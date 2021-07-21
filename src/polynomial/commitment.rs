use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::CommonCircuitData;
use crate::context;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::fri::{prover::fri_proof, verifier::verify_fri_proof};
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::{Challenger, RecursiveChallenger};
use crate::plonk_common::PlonkPolynomials;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::{FriProof, FriProofTarget, Hash, HashTarget, OpeningSet, OpeningSetTarget};
use crate::timed;
use crate::util::scaling::ReducingFactor;
use crate::util::{log2_ceil, log2_strict, reverse_bits, reverse_index_bits_in_place, transpose};

/// Two (~64 bit) field elements gives ~128 bit security.
pub const SALT_SIZE: usize = 2;

pub struct ListPolynomialCommitment<F: Field> {
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub merkle_tree: MerkleTree<F>,
    pub degree: usize,
    pub degree_log: usize,
    pub rate_bits: usize,
    pub blinding: bool,
}

impl<F: Field> ListPolynomialCommitment<F> {
    /// Creates a list polynomial commitment for the polynomials interpolating the values in `values`.
    pub fn new(values: Vec<PolynomialValues<F>>, rate_bits: usize, blinding: bool) -> Self {
        let degree = values[0].len();
        let polynomials = values.par_iter().map(|v| v.ifft()).collect::<Vec<_>>();
        let lde_values = timed!(
            Self::lde_values(&polynomials, rate_bits, blinding),
            "to compute LDE"
        );

        Self::new_from_data(polynomials, lde_values, degree, rate_bits, blinding)
    }

    /// Creates a list polynomial commitment for the polynomials `polynomials`.
    pub fn new_from_polys(
        polynomials: Vec<PolynomialCoeffs<F>>,
        rate_bits: usize,
        blinding: bool,
    ) -> Self {
        let degree = polynomials[0].len();
        let lde_values = timed!(
            Self::lde_values(&polynomials, rate_bits, blinding),
            "to compute LDE"
        );

        Self::new_from_data(polynomials, lde_values, degree, rate_bits, blinding)
    }

    fn new_from_data(
        polynomials: Vec<PolynomialCoeffs<F>>,
        lde_values: Vec<Vec<F>>,
        degree: usize,
        rate_bits: usize,
        blinding: bool,
    ) -> Self {
        let mut leaves = timed!(transpose(&lde_values), "to transpose LDEs");
        reverse_index_bits_in_place(&mut leaves);
        let merkle_tree = timed!(MerkleTree::new(leaves, false), "to build Merkle tree");

        Self {
            polynomials,
            merkle_tree,
            degree,
            degree_log: log2_strict(degree),
            rate_bits,
            blinding,
        }
    }

    fn lde_values(
        polynomials: &[PolynomialCoeffs<F>],
        rate_bits: usize,
        blinding: bool,
    ) -> Vec<Vec<F>> {
        let degree = polynomials[0].len();
        polynomials
            .par_iter()
            .map(|p| {
                assert_eq!(p.len(), degree, "Polynomial degree invalid.");
                p.lde(rate_bits).coset_fft(F::coset_shift()).values
            })
            .chain(if blinding {
                // If blinding, salt with two random elements to each leaf vector.
                (0..SALT_SIZE)
                    .map(|_| F::rand_vec(degree << rate_bits))
                    .collect()
            } else {
                Vec::new()
            })
            .collect()
    }

    pub fn get_lde_values(&self, index: usize) -> &[F] {
        let index = reverse_bits(index, self.degree_log + self.rate_bits);
        let slice = &self.merkle_tree.leaves[index];
        &slice[..slice.len() - if self.blinding { SALT_SIZE } else { 0 }]
    }

    /// Takes the commitments to the constants - sigmas - wires - zs - quotient — polynomials,
    /// and an opening point `zeta` and produces a batched opening proof + opening set.
    pub fn open_plonk<const D: usize>(
        commitments: &[&Self; 4],
        zeta: F::Extension,
        challenger: &mut Challenger<F>,
        common_data: &CommonCircuitData<F, D>,
    ) -> (OpeningProof<F, D>, OpeningSet<F, D>)
    where
        F: Extendable<D>,
    {
        let config = &common_data.config;
        assert!(D > 1, "Not implemented for D=1.");
        let degree_log = commitments[0].degree_log;
        let g = F::Extension::primitive_root_of_unity(degree_log);
        for p in &[zeta, g * zeta] {
            assert_ne!(
                p.exp(1 << degree_log as u64),
                F::Extension::ONE,
                "Opening point is in the subgroup."
            );
        }

        let os = OpeningSet::new(
            zeta,
            g,
            commitments[0],
            commitments[1],
            commitments[2],
            commitments[3],
            common_data,
        );
        challenger.observe_opening_set(&os);

        let alpha = challenger.get_extension_challenge();
        let mut alpha = ReducingFactor::new(alpha);

        // Final low-degree polynomial that goes into FRI.
        let mut final_poly = PolynomialCoeffs::empty();

        let mut zs_polys = commitments[PlonkPolynomials::ZS_PARTIAL_PRODUCTS.index]
            .polynomials
            .iter()
            .map(|p| p.to_extension())
            .collect::<Vec<_>>();
        let partial_products_polys = zs_polys.split_off(common_data.zs_range().end);

        // Polynomials opened at a single point.
        let single_polys = [
            PlonkPolynomials::CONSTANTS_SIGMAS,
            PlonkPolynomials::WIRES,
            PlonkPolynomials::QUOTIENT,
        ]
        .iter()
        .flat_map(|&p| &commitments[p.index].polynomials)
        .map(|p| p.to_extension())
        .chain(partial_products_polys);
        let single_composition_poly = alpha.reduce_polys(single_polys);

        let single_quotient = Self::compute_quotient([zeta], single_composition_poly);
        final_poly += single_quotient;
        alpha.reset();

        // Zs polynomials are opened at `zeta` and `g*zeta`.
        let zs_composition_poly = alpha.reduce_polys(zs_polys.into_iter());

        let zs_quotient = Self::compute_quotient([zeta, g * zeta], zs_composition_poly);
        alpha.shift_poly(&mut final_poly);
        final_poly += zs_quotient;

        let lde_final_poly = final_poly.lde(config.rate_bits);
        let lde_final_values = lde_final_poly.coset_fft(F::coset_shift().into());

        let fri_proof = fri_proof(
            &commitments
                .par_iter()
                .map(|c| &c.merkle_tree)
                .collect::<Vec<_>>(),
            lde_final_poly,
            lde_final_values,
            challenger,
            &config.fri_config,
        );

        (
            OpeningProof {
                fri_proof,
                quotient_degree: final_poly.len(),
            },
            os,
        )
    }

    /// Given `points=(x_i)`, `evals=(y_i)` and `poly=P` with `P(x_i)=y_i`, computes the polynomial
    /// `Q=(P-I)/Z` where `I` interpolates `(x_i, y_i)` and `Z` is the vanishing polynomial on `(x_i)`.
    fn compute_quotient<const D: usize, const N: usize>(
        points: [F::Extension; N],
        poly: PolynomialCoeffs<F::Extension>,
    ) -> PolynomialCoeffs<F::Extension>
    where
        F: Extendable<D>,
    {
        let quotient = if N == 1 {
            poly.divide_by_linear(points[0]).0
        } else if N == 2 {
            // The denominator is `(X - p0)(X - p1) = p0 p1 - (p0 + p1) X + X^2`.
            let denominator = vec![
                points[0] * points[1],
                -points[0] - points[1],
                F::Extension::ONE,
            ]
            .into();
            poly.div_rem_long_division(&denominator).0 // Could also use `divide_by_linear` twice.
        } else {
            unreachable!("This shouldn't happen. Plonk should open polynomials at 1 or 2 points.")
        };

        quotient.padded(quotient.degree_plus_one().next_power_of_two())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct OpeningProof<F: Extendable<D>, const D: usize> {
    pub(crate) fri_proof: FriProof<F, D>,
    // TODO: Get the degree from `CommonCircuitData` instead.
    quotient_degree: usize,
}

impl<F: Extendable<D>, const D: usize> OpeningProof<F, D> {
    pub fn verify(
        &self,
        zeta: F::Extension,
        os: &OpeningSet<F, D>,
        merkle_roots: &[Hash<F>],
        challenger: &mut Challenger<F>,
        common_data: &CommonCircuitData<F, D>,
    ) -> Result<()> {
        challenger.observe_opening_set(os);

        let alpha = challenger.get_extension_challenge();

        verify_fri_proof(
            log2_strict(self.quotient_degree),
            &os,
            zeta,
            alpha,
            merkle_roots,
            &self.fri_proof,
            challenger,
            common_data,
        )
    }
}

pub struct OpeningProofTarget<const D: usize> {
    pub(crate) fri_proof: FriProofTarget<D>,
}

impl<const D: usize> OpeningProofTarget<D> {
    pub fn verify<F: Extendable<D>>(
        &self,
        zeta: ExtensionTarget<D>,
        os: &OpeningSetTarget<D>,
        merkle_roots: &[HashTarget],
        challenger: &mut RecursiveChallenger,
        common_data: &CommonCircuitData<F, D>,
        builder: &mut CircuitBuilder<F, D>,
    ) {
        challenger.observe_opening_set(os);

        let alpha = challenger.get_extension_challenge(builder);

        context!(
            builder,
            "verify FRI proof",
            builder.verify_fri_proof(
                log2_ceil(common_data.degree()),
                &os,
                zeta,
                alpha,
                merkle_roots,
                &self.fri_proof,
                challenger,
                common_data,
            )
        );
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::fri::FriConfig;
    use crate::plonk_common::PlonkPolynomials;

    fn gen_random_test_case<F: Field + Extendable<D>, const D: usize>(
        k: usize,
        degree_log: usize,
    ) -> Vec<PolynomialValues<F>> {
        let degree = 1 << degree_log;

        (0..k)
            .map(|_| PolynomialValues::new(F::rand_vec(degree)))
            .collect()
    }

    fn gen_random_point<F: Field + Extendable<D>, const D: usize>(
        degree_log: usize,
    ) -> F::Extension {
        let degree = 1 << degree_log;

        let mut point = F::Extension::rand();
        while point.exp(degree as u64).is_one() {
            point = F::Extension::rand();
        }

        point
    }

    fn check_batch_polynomial_commitment<F: Field + Extendable<D>, const D: usize>() -> Result<()> {
        let ks = [10, 2, 10, 8];
        let degree_log = 11;
        let fri_config = FriConfig {
            proof_of_work_bits: 2,
            reduction_arity_bits: vec![2, 3, 1, 2],
            num_query_rounds: 3,
        };
        // We only care about `fri_config, num_constants`, and `num_routed_wires` here.
        let common_data = CommonCircuitData {
            config: CircuitConfig {
                fri_config,
                num_routed_wires: 6,
                ..CircuitConfig::large_config()
            },
            degree_bits: 0,
            gates: vec![],
            quotient_degree_factor: 0,
            num_gate_constraints: 0,
            num_constants: 4,
            k_is: vec![F::ONE; 6],
            num_partial_products: (0, 0),
            circuit_digest: Hash::from_partial(vec![]),
        };

        let lpcs = (0..4)
            .map(|i| {
                ListPolynomialCommitment::<F>::new(
                    gen_random_test_case(ks[i], degree_log),
                    common_data.config.rate_bits,
                    PlonkPolynomials::polynomials(i).blinding,
                )
            })
            .collect::<Vec<_>>();

        let zeta = gen_random_point::<F, D>(degree_log);
        let (proof, os) = ListPolynomialCommitment::open_plonk::<D>(
            &[&lpcs[0], &lpcs[1], &lpcs[2], &lpcs[3]],
            zeta,
            &mut Challenger::new(),
            &common_data,
        );

        proof.verify(
            zeta,
            &os,
            &[
                lpcs[0].merkle_tree.root,
                lpcs[1].merkle_tree.root,
                lpcs[2].merkle_tree.root,
                lpcs[3].merkle_tree.root,
            ],
            &mut Challenger::new(),
            &common_data,
        )
    }

    mod quadratic {
        use super::*;
        use crate::field::crandall_field::CrandallField;

        #[test]
        fn test_batch_polynomial_commitment() -> Result<()> {
            check_batch_polynomial_commitment::<CrandallField, 2>()
        }
    }

    mod quartic {
        use super::*;
        use crate::field::crandall_field::CrandallField;

        #[test]
        fn test_batch_polynomial_commitment() -> Result<()> {
            check_batch_polynomial_commitment::<CrandallField, 4>()
        }
    }
}
