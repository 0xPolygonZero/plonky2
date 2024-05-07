#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};

use itertools::Itertools;
use plonky2_field::extension::Extendable;
use plonky2_field::fft::FftRootTable;
use plonky2_field::packed::PackedField;
use plonky2_field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2_field::types::Field;
use plonky2_maybe_rayon::*;
use plonky2_util::{log2_strict, reverse_index_bits_in_place};

use crate::fri::batch_prover::batch_fri_proof;
use crate::fri::oracle::PolynomialBatch;
use crate::fri::proof::FriProof;
use crate::fri::structure::{FriBatchInfo, FriInstanceInfo};
use crate::fri::FriParams;
use crate::hash::field_merkle_tree::FieldMerkleTree;
use crate::hash::hash_types::RichField;
use crate::iop::challenger::Challenger;
use crate::plonk::config::GenericConfig;
use crate::timed;
use crate::util::reducing::ReducingFactor;
use crate::util::timing::TimingTree;
use crate::util::{reverse_bits, transpose};

/// Represents a batch FRI oracle, i.e. a batch of polynomials with different degrees which have
/// been Merkle-ized in a Field Merkle Tree.
#[derive(Eq, PartialEq, Debug)]
pub struct BatchFriOracle<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
{
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub field_merkle_tree: FieldMerkleTree<F, C::Hasher>,
    // The degree bits of each polynomial group.
    pub degree_bits: Vec<usize>,
    pub rate_bits: usize,
    pub blinding: bool,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    BatchFriOracle<F, C, D>
{
    /// Creates a list polynomial commitment for the polynomials interpolating the values in `values`.
    pub fn from_values(
        values: Vec<PolynomialValues<F>>,
        rate_bits: usize,
        blinding: bool,
        cap_height: usize,
        timing: &mut TimingTree,
        fft_root_table: &[Option<&FftRootTable<F>>],
    ) -> Self {
        let coeffs = timed!(
            timing,
            "IFFT",
            values.into_par_iter().map(|v| v.ifft()).collect::<Vec<_>>()
        );

        Self::from_coeffs(
            coeffs,
            rate_bits,
            blinding,
            cap_height,
            timing,
            fft_root_table,
        )
    }

    /// Creates a list polynomial commitment for the polynomials `polynomials`.
    pub fn from_coeffs(
        polynomials: Vec<PolynomialCoeffs<F>>,
        rate_bits: usize,
        blinding: bool,
        cap_height: usize,
        timing: &mut TimingTree,
        fft_root_table: &[Option<&FftRootTable<F>>],
    ) -> Self {
        let mut degree_bits = polynomials
            .iter()
            .map(|p| log2_strict(p.len()))
            .collect_vec();
        assert!(degree_bits.windows(2).all(|pair| { pair[0] >= pair[1] }));

        let num_polynomials = polynomials.len();
        let mut group_start = 0;
        let mut leaves = Vec::new();

        for (i, d) in degree_bits.iter().enumerate() {
            if i == num_polynomials - 1 || *d > degree_bits[i + 1] {
                let lde_values = timed!(
                    timing,
                    "FFT + blinding",
                    PolynomialBatch::<F, C, D>::lde_values(
                        &polynomials[group_start..i + 1],
                        rate_bits,
                        blinding,
                        fft_root_table[i]
                    )
                );

                let mut leaf_group = timed!(timing, "transpose LDEs", transpose(&lde_values));
                reverse_index_bits_in_place(&mut leaf_group);
                leaves.push(leaf_group);

                group_start = i + 1;
            }
        }

        let field_merkle_tree = timed!(
            timing,
            "build Field Merkle tree",
            FieldMerkleTree::new(leaves, cap_height)
        );

        degree_bits.sort_unstable();
        degree_bits.dedup();
        degree_bits.reverse();
        assert_eq!(field_merkle_tree.leaves.len(), degree_bits.len());
        Self {
            polynomials,
            field_merkle_tree,
            degree_bits,
            rate_bits,
            blinding,
        }
    }

    /// Produces a batch opening proof.
    pub fn prove_openings(
        degree_bits: &[usize],
        instances: &[FriInstanceInfo<F, D>],
        oracles: &[&Self],
        challenger: &mut Challenger<F, C::Hasher>,
        fri_params: &FriParams,
        timing: &mut TimingTree,
    ) -> FriProof<F, C::Hasher, D> {
        assert_eq!(degree_bits.len(), instances.len());
        assert!(D > 1, "Not implemented for D=1.");
        let alpha = challenger.get_extension_challenge::<D>();
        let mut alpha = ReducingFactor::new(alpha);

        let mut final_lde_polynomial_coeff = Vec::with_capacity(instances.len());
        let mut final_lde_polynomial_values = Vec::with_capacity(instances.len());
        for (i, instance) in instances.iter().enumerate() {
            // Final low-degree polynomial that goes into FRI.
            let mut final_poly = PolynomialCoeffs::empty();

            // Each batch `i` consists of an opening point `z_i` and polynomials `{f_ij}_j` to be opened at that point.
            // For each batch, we compute the composition polynomial `F_i = sum alpha^j f_ij`,
            // where `alpha` is a random challenge in the extension field.
            // The final polynomial is then computed as `final_poly = sum_i alpha^(k_i) (F_i(X) - F_i(z_i))/(X-z_i)`
            // where the `k_i`s are chosen such that each power of `alpha` appears only once in the final sum.
            // There are usually two batches for the openings at `zeta` and `g * zeta`.
            // The oracles used in Plonky2 are given in `FRI_ORACLES` in `plonky2/src/plonk/plonk_common.rs`.
            for FriBatchInfo { point, polynomials } in &instance.batches {
                // Collect the coefficients of all the polynomials in `polynomials`.
                let polys_coeff = polynomials.iter().map(|fri_poly| {
                    &oracles[fri_poly.oracle_index].polynomials[fri_poly.polynomial_index]
                });
                let composition_poly = timed!(
                    timing,
                    &format!("reduce batch of {} polynomials", polynomials.len()),
                    alpha.reduce_polys_base(polys_coeff)
                );
                let mut quotient = composition_poly.divide_by_linear(*point);
                quotient.coeffs.push(F::Extension::ZERO); // pad back to power of two
                alpha.shift_poly(&mut final_poly);
                final_poly += quotient;
            }

            assert_eq!(final_poly.len(), 1 << degree_bits[i]);
            let lde_final_poly = final_poly.lde(fri_params.config.rate_bits);
            let lde_final_values = timed!(
                timing,
                &format!("perform final FFT {}", lde_final_poly.len()),
                lde_final_poly.coset_fft(F::coset_shift().into())
            );
            final_lde_polynomial_coeff.push(lde_final_poly);
            final_lde_polynomial_values.push(lde_final_values);
        }

        batch_fri_proof::<F, C, D>(
            &oracles
                .iter()
                .map(|o| &o.field_merkle_tree)
                .collect::<Vec<_>>(),
            final_lde_polynomial_coeff[0].clone(),
            &final_lde_polynomial_values,
            challenger,
            fri_params,
            timing,
        )
    }

    /// Fetches LDE values at the `index * step`th point.
    pub fn get_lde_values(
        &self,
        degree_bits_index: usize,
        index: usize,
        step: usize,
        slice_start: usize,
        slice_len: usize,
    ) -> &[F] {
        let index = index * step;
        let index = reverse_bits(index, self.degree_bits[degree_bits_index] + self.rate_bits);
        let slice = &self.field_merkle_tree.leaves[degree_bits_index][index];
        &slice[slice_start..slice_start + slice_len]
    }

    /// Like `get_lde_values`, but fetches LDE values from a batch of `P::WIDTH` points, and returns
    /// packed values.
    pub fn get_lde_values_packed<P>(
        &self,
        degree_bits_index: usize,
        index_start: usize,
        step: usize,
        slice_start: usize,
        slice_len: usize,
    ) -> Vec<P>
    where
        P: PackedField<Scalar = F>,
    {
        let row_wise = (0..P::WIDTH)
            .map(|i| {
                self.get_lde_values(
                    degree_bits_index,
                    index_start + i,
                    step,
                    slice_start,
                    slice_len,
                )
            })
            .collect_vec();

        // This is essentially a transpose, but we will not use the generic transpose method as we
        // want inner lists to be of type P, not Vecs which would involve allocation.
        let leaf_size = row_wise[0].len();
        (0..leaf_size)
            .map(|j| {
                let mut packed = P::ZEROS;
                packed
                    .as_slice_mut()
                    .iter_mut()
                    .zip(&row_wise)
                    .for_each(|(packed_i, row_i)| *packed_i = row_i[j]);
                packed
            })
            .collect_vec()
    }
}

#[cfg(test)]
mod test {
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::types::Sample;

    use super::*;
    use crate::fri::batch_oracle::BatchFriOracle;
    use crate::fri::batch_verifier::verify_batch_fri_proof;
    use crate::fri::reduction_strategies::FriReductionStrategy;
    use crate::fri::structure::{
        FriBatchInfo, FriInstanceInfo, FriOpeningBatch, FriOpenings, FriOracleInfo,
        FriPolynomialInfo,
    };
    use crate::fri::FriConfig;
    use crate::plonk::config::PoseidonGoldilocksConfig;

    const D: usize = 2;

    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    type H = <C as GenericConfig<D>>::Hasher;

    #[test]
    fn batch_prove_openings() -> anyhow::Result<()> {
        let mut timing = TimingTree::default();

        let k0 = 9;
        let k1 = 8;
        let k2 = 6;
        let reduction_arity_bits = vec![1, 2, 1];
        let fri_params = FriParams {
            config: FriConfig {
                rate_bits: 1,
                cap_height: 0,
                proof_of_work_bits: 0,
                reduction_strategy: FriReductionStrategy::Fixed(reduction_arity_bits.clone()),
                num_query_rounds: 10,
            },
            hiding: false,
            degree_bits: k0,
            reduction_arity_bits,
        };

        let n0 = 1 << k0;
        let n1 = 1 << k1;
        let n2 = 1 << k2;
        let trace0 = PolynomialValues::new(F::rand_vec(n0));
        let trace1_0 = PolynomialValues::new(F::rand_vec(n1));
        let trace1_1 = PolynomialValues::new(F::rand_vec(n1));
        let trace2 = PolynomialValues::new(F::rand_vec(n2));

        let trace_oracle: BatchFriOracle<GoldilocksField, C, D> = BatchFriOracle::from_values(
            vec![
                trace0.clone(),
                trace1_0.clone(),
                trace1_1.clone(),
                trace2.clone(),
            ],
            fri_params.config.rate_bits,
            fri_params.hiding,
            fri_params.config.cap_height,
            &mut timing,
            &[None; 4],
        );

        let mut challenger = Challenger::<F, H>::new();
        challenger.observe_cap(&trace_oracle.field_merkle_tree.cap);
        let zeta = challenger.get_extension_challenge::<D>();
        let eta = challenger.get_extension_challenge::<D>();
        let poly0 = &trace_oracle.polynomials[0];
        let poly1_0 = &trace_oracle.polynomials[1];
        let poly1_1 = &trace_oracle.polynomials[2];
        let poly2 = &trace_oracle.polynomials[3];
        let mut verifier_challenger = challenger.clone();

        let fri_instance_0 = FriInstanceInfo {
            oracles: vec![FriOracleInfo {
                num_polys: 1,
                blinding: false,
            }],
            batches: vec![
                FriBatchInfo {
                    point: zeta,
                    polynomials: vec![FriPolynomialInfo {
                        oracle_index: 0,
                        polynomial_index: 0,
                    }],
                },
                FriBatchInfo {
                    point: eta,
                    polynomials: vec![FriPolynomialInfo {
                        oracle_index: 0,
                        polynomial_index: 0,
                    }],
                },
            ],
        };
        let fri_instance_1 = FriInstanceInfo {
            oracles: vec![FriOracleInfo {
                num_polys: 2,
                blinding: false,
            }],
            batches: vec![
                FriBatchInfo {
                    point: zeta,
                    polynomials: vec![
                        FriPolynomialInfo {
                            oracle_index: 0,
                            polynomial_index: 1,
                        },
                        FriPolynomialInfo {
                            oracle_index: 0,
                            polynomial_index: 2,
                        },
                    ],
                },
                FriBatchInfo {
                    point: eta,
                    polynomials: vec![FriPolynomialInfo {
                        oracle_index: 0,
                        polynomial_index: 2,
                    }],
                },
            ],
        };
        let fri_instance_2 = FriInstanceInfo {
            oracles: vec![FriOracleInfo {
                num_polys: 1,
                blinding: false,
            }],
            batches: vec![FriBatchInfo {
                point: zeta,
                polynomials: vec![FriPolynomialInfo {
                    oracle_index: 0,
                    polynomial_index: 3,
                }],
            }],
        };
        let fri_instances = vec![fri_instance_0, fri_instance_1, fri_instance_2];
        let fri_opening_batch_0 = FriOpenings {
            batches: vec![
                FriOpeningBatch {
                    values: vec![poly0.to_extension::<D>().eval(zeta)],
                },
                FriOpeningBatch {
                    values: vec![poly0.to_extension::<D>().eval(eta)],
                },
            ],
        };
        let fri_opening_batch_1 = FriOpenings {
            batches: vec![
                FriOpeningBatch {
                    values: vec![
                        poly1_0.to_extension::<D>().eval(zeta),
                        poly1_1.to_extension::<D>().eval(zeta),
                    ],
                },
                FriOpeningBatch {
                    values: vec![poly1_1.to_extension::<D>().eval(eta)],
                },
            ],
        };
        let fri_opening_batch_2 = FriOpenings {
            batches: vec![FriOpeningBatch {
                values: vec![poly2.to_extension::<D>().eval(zeta)],
            }],
        };
        let fri_openings = vec![
            fri_opening_batch_0,
            fri_opening_batch_1,
            fri_opening_batch_2,
        ];

        let proof = BatchFriOracle::prove_openings(
            &[k0, k1, k2],
            &fri_instances,
            &[&trace_oracle],
            &mut challenger,
            &fri_params,
            &mut timing,
        );

        let fri_challenges = verifier_challenger.fri_challenges::<C, D>(
            &proof.commit_phase_merkle_caps,
            &proof.final_poly,
            proof.pow_witness,
            k0,
            &fri_params.config,
        );
        verify_batch_fri_proof::<GoldilocksField, C, D>(
            &[k0, k1, k2],
            &fri_instances,
            &fri_openings,
            &fri_challenges,
            &[trace_oracle.field_merkle_tree.cap],
            &proof,
            &fri_params,
        )
    }
}
