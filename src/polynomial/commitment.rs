use anyhow::Result;
use rayon::prelude::*;

use crate::field::extension_field::Extendable;
use crate::field::extension_field::{FieldExtension, Frobenius};
use crate::field::field::Field;
use crate::field::lagrange::interpolant;
use crate::fri::{prover::fri_proof, verifier::verify_fri_proof, FriConfig};
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::{reduce_polys_with_iter, reduce_with_iter};
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::proof::{FriProof, FriProofTarget, Hash, OpeningSet};
use crate::timed;
use crate::util::{log2_strict, reverse_index_bits_in_place, transpose};

pub const SALT_SIZE: usize = 2;

pub struct ListPolynomialCommitment<F: Field> {
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub merkle_tree: MerkleTree<F>,
    pub degree: usize,
    pub rate_bits: usize,
    pub blinding: bool,
}

impl<F: Field> ListPolynomialCommitment<F> {
    pub fn new(polynomials: Vec<PolynomialCoeffs<F>>, rate_bits: usize, blinding: bool) -> Self {
        let degree = polynomials[0].len();
        let lde_values = timed!(
            Self::lde_values(&polynomials, rate_bits, blinding),
            "to compute LDE"
        );

        let mut leaves = timed!(transpose(&lde_values), "to transpose LDEs");
        reverse_index_bits_in_place(&mut leaves);
        let merkle_tree = timed!(MerkleTree::new(leaves, false), "to build Merkle tree");

        Self {
            polynomials,
            merkle_tree,
            degree,
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
                p.clone()
                    .lde(rate_bits)
                    .coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR)
                    .values
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

    pub fn leaf(&self, index: usize) -> &[F] {
        let leaf = &self.merkle_tree.leaves[index];
        &leaf[0..leaf.len() - if self.blinding { SALT_SIZE } else { 0 }]
    }

    /// Takes the commitments to the constants - sigmas - wires - zs - quotient — polynomials,
    /// and an opening point `zeta` and produces a batched opening proof + opening set.
    pub fn open_plonk<const D: usize>(
        commitments: &[&Self; 5],
        zeta: F::Extension,
        challenger: &mut Challenger<F>,
        config: &FriConfig,
    ) -> (OpeningProof<F, D>, OpeningSet<F, D>)
    where
        F: Extendable<D>,
    {
        assert!(D > 1, "Not implemented for D=1.");
        let degree_log = log2_strict(commitments[0].degree);
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
            commitments[4],
        );
        challenger.observe_opening_set(&os);

        let alpha = challenger.get_extension_challenge();
        let mut alpha_powers = alpha.powers();

        // Final low-degree polynomial that goes into FRI.
        let mut final_poly = PolynomialCoeffs::empty();

        // Polynomials opened at a single point.
        let single_polys = [0, 1, 4]
            .iter()
            .flat_map(|&i| &commitments[i].polynomials)
            .map(|p| p.to_extension());
        let single_os = [&os.constants, &os.plonk_s_sigmas, &os.quotient_polys];
        let single_evals = single_os.iter().flat_map(|v| v.iter());
        let single_composition_poly = reduce_polys_with_iter(single_polys, alpha_powers.clone());
        let single_composition_eval = reduce_with_iter(single_evals, &mut alpha_powers);

        let single_quotient = Self::compute_quotient(
            &[zeta],
            &[single_composition_eval],
            &single_composition_poly,
        );
        final_poly = &final_poly + &single_quotient;

        // Zs polynomials are opened at `zeta` and `g*zeta`.
        let zs_polys = commitments[3].polynomials.iter().map(|p| p.to_extension());
        let zs_composition_poly = reduce_polys_with_iter(zs_polys, alpha_powers.clone());
        let zs_composition_evals = [
            reduce_with_iter(&os.plonk_zs, alpha_powers.clone()),
            reduce_with_iter(&os.plonk_zs_right, &mut alpha_powers),
        ];

        let zs_quotient = Self::compute_quotient(
            &[zeta, g * zeta],
            &zs_composition_evals,
            &zs_composition_poly,
        );
        final_poly = &final_poly + &zs_quotient;

        // When working in an extension field, need to check that wires are in the base field.
        // Check this by opening the wires polynomials at `zeta` and `zeta.frobenius()` and using the fact that
        // a polynomial `f` is over the base field iff `f(z).frobenius()=f(z.frobenius())` with high probability.
        let wire_polys = commitments[2].polynomials.iter().map(|p| p.to_extension());
        let wire_composition_poly = reduce_polys_with_iter(wire_polys, alpha_powers.clone());
        let wire_evals_frob = os.wires.iter().map(|e| e.frobenius()).collect::<Vec<_>>();
        let wire_composition_evals = [
            reduce_with_iter(&os.wires, alpha_powers.clone()),
            reduce_with_iter(&wire_evals_frob, alpha_powers),
        ];

        let wires_quotient = Self::compute_quotient(
            &[zeta, zeta.frobenius()],
            &wire_composition_evals,
            &wire_composition_poly,
        );
        final_poly = &final_poly + &wires_quotient;

        let lde_final_poly = final_poly.lde(config.rate_bits);
        let lde_final_values = lde_final_poly
            .clone()
            .coset_fft(F::Extension::from_basefield(
                F::MULTIPLICATIVE_GROUP_GENERATOR,
            ));

        let fri_proof = fri_proof(
            &commitments
                .par_iter()
                .map(|c| &c.merkle_tree)
                .collect::<Vec<_>>(),
            &lde_final_poly,
            &lde_final_values,
            challenger,
            &config,
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
    fn compute_quotient<const D: usize>(
        points: &[F::Extension],
        evals: &[F::Extension],
        poly: &PolynomialCoeffs<F::Extension>,
    ) -> PolynomialCoeffs<F::Extension>
    where
        F: Extendable<D>,
    {
        let pairs = points
            .iter()
            .zip(evals)
            .map(|(&x, &e)| (x, e))
            .collect::<Vec<_>>();
        debug_assert!(pairs.iter().all(|&(x, e)| poly.eval(x) == e));

        let interpolant = interpolant(&pairs);
        let denominator = points.iter().fold(PolynomialCoeffs::one(), |acc, &x| {
            &acc * &PolynomialCoeffs::new(vec![-x, F::Extension::ONE])
        });
        let numerator = poly - &interpolant;
        let (quotient, rem) = numerator.div_rem(&denominator);
        debug_assert!(rem.is_zero());

        quotient.padded(quotient.degree_plus_one().next_power_of_two())
    }
}

pub struct OpeningProof<F: Field + Extendable<D>, const D: usize> {
    fri_proof: FriProof<F, D>,
    // TODO: Get the degree from `CommonCircuitData` instead.
    quotient_degree: usize,
}

impl<F: Field + Extendable<D>, const D: usize> OpeningProof<F, D> {
    pub fn verify(
        &self,
        zeta: F::Extension,
        os: &OpeningSet<F, D>,
        merkle_roots: &[Hash<F>],
        challenger: &mut Challenger<F>,
        fri_config: &FriConfig,
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
            fri_config,
        )
    }
}

pub struct OpeningProofTarget<const D: usize> {
    fri_proof: FriProofTarget<D>,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::plonk_common::PlonkPolynomials;

    fn gen_random_test_case<F: Field + Extendable<D>, const D: usize>(
        k: usize,
        degree_log: usize,
    ) -> Vec<PolynomialCoeffs<F>> {
        let degree = 1 << degree_log;

        (0..k)
            .map(|_| PolynomialCoeffs::new(F::rand_vec(degree)))
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
        let ks = [1, 2, 3, 5, 8];
        let degree_log = 11;
        let fri_config = FriConfig {
            proof_of_work_bits: 2,
            rate_bits: 2,
            reduction_arity_bits: vec![2, 3, 1, 2],
            num_query_rounds: 3,
        };

        let lpcs = (0..5)
            .map(|i| {
                ListPolynomialCommitment::<F>::new(
                    gen_random_test_case(ks[i], degree_log),
                    fri_config.rate_bits,
                    PlonkPolynomials::polynomials(i).blinding,
                )
            })
            .collect::<Vec<_>>();

        let zeta = gen_random_point::<F, D>(degree_log);
        let (proof, os) = ListPolynomialCommitment::open_plonk::<D>(
            &[&lpcs[0], &lpcs[1], &lpcs[2], &lpcs[3], &lpcs[4]],
            zeta,
            &mut Challenger::new(),
            &fri_config,
        );

        proof.verify(
            zeta,
            &os,
            &[
                lpcs[0].merkle_tree.root,
                lpcs[1].merkle_tree.root,
                lpcs[2].merkle_tree.root,
                lpcs[3].merkle_tree.root,
                lpcs[4].merkle_tree.root,
            ],
            &mut Challenger::new(),
            &fri_config,
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
