#[cfg(not(feature = "std"))]
use alloc::vec;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use plonky2_field::extension::flatten;
use plonky2_maybe_rayon::*;
use plonky2_util::reverse_index_bits_in_place;

use crate::field::extension::{unflatten, Extendable};
use crate::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::fri::proof::{FriInitialTreeProof, FriProof, FriQueryRound, FriQueryStep};
use crate::fri::prover::fri_proof_of_work;
use crate::fri::FriParams;
use crate::hash::field_merkle_tree::FieldMerkleTree;
use crate::hash::hash_types::RichField;
use crate::hash::merkle_tree::MerkleTree;
use crate::iop::challenger::Challenger;
use crate::plonk::config::GenericConfig;
use crate::plonk::plonk_common::reduce_with_powers;
use crate::timed;
use crate::util::timing::TimingTree;

/// Builds a batch FRI proof.
pub fn batch_fri_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    initial_merkle_trees: &FieldMerkleTree<F, C::Hasher>,
    lde_polynomial_coeffs: &mut [PolynomialCoeffs<F::Extension>],
    // lde_polynomial_values: &mut Vec<PolynomialValues<F::Extension>>,
    lde_polynomial_values: PolynomialValues<F::Extension>,
    challenger: &mut Challenger<F, C::Hasher>,
    fri_params: &FriParams,
    timing: &mut TimingTree,
) -> FriProof<F, C::Hasher, D> {
    let n = lde_polynomial_values.len();
    assert_eq!(lde_polynomial_coeffs[0].len(), n);
    // The polynomial vectors should be sorted by degree, from largest to smallest, with no duplicate degrees.
    assert!(lde_polynomial_coeffs
        .windows(2)
        .all(|pair| { pair[0].len() > pair[1].len() }));

    // Commit phase
    let (trees, final_coeffs) = timed!(
        timing,
        "fold codewords in the commitment phase",
        batch_fri_committed_trees::<F, C, D>(
            lde_polynomial_coeffs,
            lde_polynomial_values,
            challenger,
            fri_params,
        )
    );

    // PoW phase
    let pow_witness = timed!(
        timing,
        "find proof-of-work witness",
        fri_proof_of_work::<F, C, D>(challenger, &fri_params.config)
    );

    // Query phase
    let query_round_proofs = batch_fri_prover_query_rounds::<F, C, D>(
        initial_merkle_trees,
        &trees,
        challenger,
        n,
        fri_params,
    );

    FriProof {
        commit_phase_merkle_caps: trees.iter().map(|t| t.cap.clone()).collect(),
        query_round_proofs,
        final_poly: final_coeffs,
        pow_witness,
    }
}

pub(crate) fn batch_fri_committed_trees<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    coeffs: &mut [PolynomialCoeffs<F::Extension>],
    mut values: PolynomialValues<F::Extension>,
    challenger: &mut Challenger<F, C::Hasher>,
    fri_params: &FriParams,
) -> crate::fri::prover::FriCommitedTrees<F, C, D> {
    let mut trees = Vec::with_capacity(fri_params.reduction_arity_bits.len());
    let mut shift = F::MULTIPLICATIVE_GROUP_GENERATOR;
    let mut polynomial_index = 1;
    let mut final_coeffs = coeffs[0].clone();
    // let mut final_values = PolynomialValues::new(vec![F::Extension::ZERO; next_fold_degree]);
    // let mut beta = F::Extension::ONE;
    for arity_bits in &fri_params.reduction_arity_bits {
        let arity = 1 << arity_bits;

        // if next_fold_degree == degree {
        //     final_values.add_assign_scaled(&values[polynomial_index], beta * beta);
        // }

        reverse_index_bits_in_place(&mut values.values);
        let chunked_values = values
            .values
            .par_chunks(arity)
            .map(|chunk: &[F::Extension]| flatten(chunk))
            .collect();
        let tree = MerkleTree::<F, C::Hasher>::new(chunked_values, fri_params.config.cap_height);

        challenger.observe_cap(&tree.cap);
        trees.push(tree);

        let beta = challenger.get_extension_challenge::<D>();
        // P(x) = sum_{i<r} x^i * P_i(x^r) becomes sum_{i<r} beta^i * P_i(x).
        final_coeffs = PolynomialCoeffs::new(
            final_coeffs
                .coeffs
                .par_chunks_exact(arity)
                .map(|chunk| reduce_with_powers(chunk, beta))
                .collect::<Vec<_>>(),
        );
        if polynomial_index != coeffs.len() && final_coeffs.len() == coeffs[polynomial_index].len()
        {
            final_coeffs = PolynomialCoeffs::new(
                final_coeffs
                    .coeffs
                    .iter()
                    .zip(&coeffs[polynomial_index].coeffs)
                    .map(|(&f, &c)| f * beta + c)
                    .collect::<Vec<_>>(),
            );
            polynomial_index += 1;
        }

        shift = shift.exp_u64(arity as u64);
        values = final_coeffs.coset_fft(shift.into());
    }
    assert_eq!(polynomial_index, coeffs.len());

    // The coefficients being removed here should always be zero.
    final_coeffs
        .coeffs
        .truncate(final_coeffs.len() >> fri_params.config.rate_bits);

    challenger.observe_extension_elements(&final_coeffs.coeffs);
    (trees, final_coeffs)
}

fn batch_fri_prover_query_rounds<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    initial_merkle_trees: &FieldMerkleTree<F, C::Hasher>,
    trees: &[MerkleTree<F, C::Hasher>],
    challenger: &mut Challenger<F, C::Hasher>,
    n: usize,
    fri_params: &FriParams,
) -> Vec<FriQueryRound<F, C::Hasher, D>> {
    challenger
        .get_n_challenges(fri_params.config.num_query_rounds)
        .into_iter()
        .map(|rand| {
            let x_index = rand.to_canonical_u64() as usize % n;
            batch_fri_prover_query_round::<F, C, D>(
                initial_merkle_trees,
                trees,
                x_index,
                fri_params,
            )
        })
        .collect()
}

fn batch_fri_prover_query_round<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    initial_merkle_trees: &FieldMerkleTree<F, C::Hasher>,
    trees: &[MerkleTree<F, C::Hasher>],
    mut x_index: usize,
    fri_params: &FriParams,
) -> FriQueryRound<F, C::Hasher, D> {
    let mut query_steps = Vec::new();
    let initial_proof = (
        initial_merkle_trees
            .values(x_index)
            .iter()
            .flatten()
            .cloned()
            .collect(),
        initial_merkle_trees.open_batch(x_index),
    );
    for (i, tree) in trees.iter().enumerate() {
        let arity_bits = fri_params.reduction_arity_bits[i];
        let evals = unflatten(tree.get(x_index >> arity_bits));
        let merkle_proof = tree.prove(x_index >> arity_bits);

        query_steps.push(FriQueryStep {
            evals,
            merkle_proof,
        });

        x_index >>= arity_bits;
    }
    FriQueryRound {
        initial_trees_proof: FriInitialTreeProof {
            evals_proofs: vec![initial_proof],
        },
        steps: query_steps,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    use anyhow::Result;
    use itertools::Itertools;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::types::{Field, Field64, Sample};

    use super::*;
    use crate::field::extension::Extendable;
    use crate::fri::batch_oracle::BatchFriOracle;
    use crate::fri::batch_verifier::verify_batch_fri_proof;
    use crate::fri::oracle::PolynomialBatch;
    use crate::fri::prover::fri_proof;
    use crate::fri::reduction_strategies::FriReductionStrategy;
    use crate::fri::structure::{
        FriBatchInfo, FriInstanceInfo, FriOpeningBatch, FriOpenings, FriOracleInfo,
        FriPolynomialInfo,
    };
    use crate::fri::verifier::verify_fri_proof;
    use crate::fri::FriConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    const D: usize = 2;

    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    type H = <C as GenericConfig<D>>::Hasher;

    #[test]
    fn single_polynomial() -> Result<()> {
        // let _ = env_logger::builder().format_timestamp(None).try_init();

        let mut timing = TimingTree::default();

        let k0 = 9;
        let reduction_arity_bits = vec![1, 2, 1];
        let fri_params = FriParams {
            config: FriConfig {
                rate_bits: 1,
                cap_height: 5,
                proof_of_work_bits: 0,
                reduction_strategy: FriReductionStrategy::Fixed(reduction_arity_bits.clone()),
                num_query_rounds: 10,
            },
            hiding: false,
            degree_bits: k0,
            reduction_arity_bits,
        };

        let n0 = 1 << k0;
        let trace00 =
            PolynomialValues::new((1..n0 + 1).map(|i| F::from_canonical_i64(i)).collect_vec());

        let polynomial_batch: PolynomialBatch<GoldilocksField, C, D> = PolynomialBatch::from_values(
            vec![trace00.clone()],
            fri_params.config.rate_bits,
            fri_params.hiding,
            fri_params.config.cap_height,
            &mut timing,
            None,
        );
        let poly = &polynomial_batch.polynomials[0];
        let mut challenger = Challenger::<F, H>::new();
        challenger.observe_cap(&polynomial_batch.merkle_tree.cap);
        let _alphas = challenger.get_n_challenges(2);
        let zeta = challenger.get_extension_challenge::<D>();
        challenger.observe_extension_element::<D>(&poly.to_extension::<D>().eval(zeta));
        let mut verfier_challenger = challenger.clone();

        let fri_instance: FriInstanceInfo<F, D> = FriInstanceInfo {
            oracles: vec![FriOracleInfo {
                num_polys: 1,
                blinding: false,
            }],
            batches: vec![FriBatchInfo {
                point: zeta,
                polynomials: vec![FriPolynomialInfo {
                    oracle_index: 0,
                    polynomial_index: 0,
                }],
            }],
        };
        let _alpha = challenger.get_extension_challenge::<D>();

        let composition_poly = poly.mul_extension::<D>(<F as Extendable<D>>::Extension::ONE);
        let mut quotient = composition_poly.divide_by_linear(zeta);
        quotient.coeffs.push(<F as Extendable<D>>::Extension::ZERO);

        let lde_final_poly = quotient.lde(fri_params.config.rate_bits);
        let lde_final_values = lde_final_poly.coset_fft(F::coset_shift().into());

        let proof = fri_proof::<F, C, D>(
            &vec![&polynomial_batch.merkle_tree],
            lde_final_poly,
            lde_final_values,
            &mut challenger,
            &fri_params,
            &mut timing,
        );

        let fri_challenges = verfier_challenger.fri_challenges::<C, D>(
            &proof.commit_phase_merkle_caps,
            &proof.final_poly,
            proof.pow_witness,
            k0,
            &fri_params.config,
        );

        let fri_opening_batch = FriOpeningBatch {
            values: vec![poly.to_extension::<D>().eval(zeta)],
        };
        verify_fri_proof::<GoldilocksField, C, 2>(
            &fri_instance,
            &FriOpenings {
                batches: vec![fri_opening_batch],
            },
            &fri_challenges,
            &[polynomial_batch.merkle_tree.cap],
            &proof,
            &fri_params,
        )
    }

    #[test]
    fn multiple_polynomials() -> Result<()> {
        // let _ = env_logger::builder().format_timestamp(None).try_init();

        let mut timing = TimingTree::default();

        let k0 = 9;
        let k1 = 8;
        let k2 = 6;
        let reduction_arity_bits = vec![1, 2, 1];
        let fri_params = FriParams {
            config: FriConfig {
                rate_bits: 1,
                cap_height: 5,
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
        let trace1 = PolynomialValues::new(F::rand_vec(n1));
        let trace2 = PolynomialValues::new(F::rand_vec(n2));

        let trace_oracle: BatchFriOracle<GoldilocksField, C, D> = BatchFriOracle::from_values(
            vec![trace0.clone(), trace1.clone(), trace2.clone()],
            fri_params.config.rate_bits,
            fri_params.hiding,
            fri_params.config.cap_height,
            &mut timing,
            &[None; 3],
        );

        let mut challenger = Challenger::<F, H>::new();
        challenger.observe_cap(&trace_oracle.field_merkle_tree.cap);
        let _alphas = challenger.get_n_challenges(2);
        let zeta = challenger.get_extension_challenge::<D>();
        let poly0 = &trace_oracle.polynomials[0];
        let poly1 = &trace_oracle.polynomials[1];
        let poly2 = &trace_oracle.polynomials[2];
        challenger.observe_extension_element::<D>(&poly0.to_extension::<D>().eval(zeta));
        challenger.observe_extension_element::<D>(&poly1.to_extension::<D>().eval(zeta));
        challenger.observe_extension_element::<D>(&poly2.to_extension::<D>().eval(zeta));
        let mut verfier_challenger = challenger.clone();

        let _alpha = challenger.get_extension_challenge::<D>();

        let composition_poly = poly0.mul_extension::<D>(<F as Extendable<D>>::Extension::ONE);
        let mut quotient = composition_poly.divide_by_linear(zeta);
        quotient.coeffs.push(<F as Extendable<D>>::Extension::ZERO);
        let lde_final_poly_0 = quotient.lde(fri_params.config.rate_bits);
        let lde_final_values = lde_final_poly_0.coset_fft(F::coset_shift().into());

        let composition_poly = poly1.mul_extension::<D>(<F as Extendable<D>>::Extension::ONE);
        let mut quotient = composition_poly.divide_by_linear(zeta);
        quotient.coeffs.push(<F as Extendable<D>>::Extension::ZERO);
        let lde_final_poly_1 = quotient.lde(fri_params.config.rate_bits);

        let composition_poly = poly2.mul_extension::<D>(<F as Extendable<D>>::Extension::ONE);
        let mut quotient = composition_poly.divide_by_linear(zeta);
        quotient.coeffs.push(<F as Extendable<D>>::Extension::ZERO);
        let lde_final_poly_2 = quotient.lde(fri_params.config.rate_bits);

        let proof = batch_fri_proof::<F, C, D>(
            &trace_oracle.field_merkle_tree,
            &mut [lde_final_poly_0, lde_final_poly_1, lde_final_poly_2],
            lde_final_values,
            &mut challenger,
            &fri_params,
            &mut timing,
        );

        let fri_instance = &FriInstanceInfo {
            oracles: vec![FriOracleInfo {
                num_polys: 1,
                blinding: false,
            }],
            batches: vec![FriBatchInfo {
                point: zeta,
                polynomials: vec![FriPolynomialInfo {
                    oracle_index: 0,
                    polynomial_index: 0,
                }],
            }],
        };
        let mut fri_instances = vec![];
        fri_instances.push(fri_instance);
        fri_instances.push(fri_instance);
        fri_instances.push(fri_instance);
        let fri_challenges = verfier_challenger.fri_challenges::<C, D>(
            &proof.commit_phase_merkle_caps,
            &proof.final_poly,
            proof.pow_witness,
            k0,
            &fri_params.config,
        );
        let fri_opening_batch_0 = &FriOpenings {
            batches: vec![FriOpeningBatch {
                values: vec![poly0.to_extension::<D>().eval(zeta)],
            }],
        };
        let fri_opening_batch_1 = &FriOpenings {
            batches: vec![FriOpeningBatch {
                values: vec![poly1.to_extension::<D>().eval(zeta)],
            }],
        };
        let fri_opening_batch_2 = &FriOpenings {
            batches: vec![FriOpeningBatch {
                values: vec![poly2.to_extension::<D>().eval(zeta)],
            }],
        };
        let mut fri_openings = vec![];
        fri_openings.push(fri_opening_batch_0);
        fri_openings.push(fri_opening_batch_1);
        fri_openings.push(fri_opening_batch_2);

        verify_batch_fri_proof::<GoldilocksField, C, 2>(
            &fri_instances,
            &fri_openings,
            &fri_challenges,
            &[trace_oracle.field_merkle_tree.cap],
            &proof,
            &fri_params,
        )
    }
}
