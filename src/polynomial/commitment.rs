use crate::field::field::Field;
use crate::field::lagrange::interpolant;
use crate::fri::{prover::fri_proof, verifier::verify_fri_proof, FriConfig};
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::reduce_with_powers;
use crate::polynomial::old_polynomial::Polynomial;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::proof::{FriProof, Hash};
use crate::util::{log2_strict, reverse_index_bits_in_place, transpose};
use anyhow::Result;

pub const SALT_SIZE: usize = 2;

struct ListPolynomialCommitment<F: Field> {
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub fri_config: FriConfig,
    pub merkle_tree: MerkleTree<F>,
    pub degree: usize,
}

impl<F: Field> ListPolynomialCommitment<F> {
    pub fn new(polynomials: Vec<PolynomialCoeffs<F>>, fri_config: &FriConfig) -> Self {
        let degree = polynomials[0].len();
        let lde_values = polynomials
            .iter()
            .map(|p| {
                assert_eq!(p.len(), degree, "Polynomial degree invalid.");
                p.clone()
                    .lde(fri_config.rate_bits)
                    .coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR)
                    .values
            })
            .chain(if fri_config.blinding {
                // If blinding, salt with two random elements to each leaf vector.
                (0..SALT_SIZE)
                    .map(|_| F::rand_vec(degree << fri_config.rate_bits))
                    .collect()
            } else {
                Vec::new()
            })
            .collect::<Vec<_>>();

        let mut leaves = transpose(&lde_values);
        reverse_index_bits_in_place(&mut leaves);
        let merkle_tree = MerkleTree::new(leaves, false);

        Self {
            polynomials,
            fri_config: fri_config.clone(),
            merkle_tree,
            degree,
        }
    }

    pub fn open(
        &self,
        points: &[F],
        challenger: &mut Challenger<F>,
    ) -> (OpeningProof<F>, Vec<Vec<F>>) {
        for p in points {
            assert_ne!(
                p.exp_usize(self.degree),
                F::ONE,
                "Opening point is in the subgroup."
            );
        }

        let evaluations = points
            .iter()
            .map(|&x| {
                self.polynomials
                    .iter()
                    .map(|p| p.eval(x))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for evals in &evaluations {
            challenger.observe_elements(evals);
        }

        let alpha = challenger.get_challenge();

        // Scale polynomials by `alpha`.
        let composition_poly = self
            .polynomials
            .iter()
            .rev()
            .map(|p| p.clone().into())
            .fold(Polynomial::empty(), |acc, p| acc.scalar_mul(alpha).add(&p));
        // Scale evaluations by `alpha`.
        let composition_evals = evaluations
            .iter()
            .map(|e| reduce_with_powers(e, alpha))
            .collect::<Vec<_>>();

        let quotient = Self::compute_quotient(points, &composition_evals, &composition_poly);

        let lde_quotient = PolynomialCoeffs::from(quotient.clone()).lde(self.fri_config.rate_bits);
        let lde_quotient_values = lde_quotient
            .clone()
            .coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR);

        let fri_proof = fri_proof(
            &[&self.merkle_tree],
            &lde_quotient,
            &lde_quotient_values,
            challenger,
            &self.fri_config,
        );

        (
            OpeningProof {
                fri_proof,
                quotient_degree: quotient.len(),
            },
            evaluations,
        )
    }

    pub fn batch_open(
        commitments: &[&Self],
        points: &[F],
        challenger: &mut Challenger<F>,
    ) -> (OpeningProof<F>, Vec<Vec<F>>) {
        let degree = commitments[0].degree;
        assert!(
            commitments.iter().all(|c| c.degree == degree),
            "Trying to open polynomial commitments of different degrees."
        );
        let fri_config = &commitments[0].fri_config;
        assert!(
            commitments.iter().all(|c| &c.fri_config == fri_config),
            "Trying to open polynomial commitments with different config."
        );
        for p in points {
            assert_ne!(
                p.exp_usize(degree),
                F::ONE,
                "Opening point is in the subgroup."
            );
        }

        let evaluations = points
            .iter()
            .map(|&x| {
                commitments
                    .iter()
                    .flat_map(move |c| c.polynomials.iter().map(|p| p.eval(x)).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for evals in &evaluations {
            challenger.observe_elements(evals);
        }

        let alpha = challenger.get_challenge();

        // Scale polynomials by `alpha`.
        let composition_poly = commitments
            .iter()
            .flat_map(|c| &c.polynomials)
            .rev()
            .map(|p| p.clone().into())
            .fold(Polynomial::empty(), |acc, p| acc.scalar_mul(alpha).add(&p));
        // Scale evaluations by `alpha`.
        let composition_evals = evaluations
            .iter()
            .map(|e| reduce_with_powers(e, alpha))
            .collect::<Vec<_>>();

        let quotient = Self::compute_quotient(points, &composition_evals, &composition_poly);

        let lde_quotient = PolynomialCoeffs::from(quotient.clone()).lde(fri_config.rate_bits);
        let lde_quotient_values = lde_quotient
            .clone()
            .coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR);

        let fri_proof = fri_proof(
            &commitments
                .iter()
                .map(|c| &c.merkle_tree)
                .collect::<Vec<_>>(),
            &lde_quotient,
            &lde_quotient_values,
            challenger,
            &fri_config,
        );

        (
            OpeningProof {
                fri_proof,
                quotient_degree: quotient.len(),
            },
            evaluations,
        )
    }

    /// Given `points=(x_i)`, `evals=(y_i)` and `poly=P` with `P(x_i)=y_i`, computes the polynomial
    /// `Q=(P-I)/Z` where `I` interpolates `(x_i, y_i)` and `Z` is the vanishing polynomial on `(x_i)`.
    fn compute_quotient(points: &[F], evals: &[F], poly: &Polynomial<F>) -> Polynomial<F> {
        let pairs = points
            .iter()
            .zip(evals)
            .map(|(&x, &e)| (x, e))
            .collect::<Vec<_>>();
        debug_assert!(pairs.iter().all(|&(x, e)| poly.eval(x) == e));

        let interpolant: Polynomial<F> = interpolant(&pairs).into();
        let denominator = points
            .iter()
            .fold(Polynomial::from(vec![F::ONE]), |acc, &x| {
                acc.mul(&vec![-x, F::ONE].into())
            });
        let numerator = poly.add(&interpolant.neg());
        let (mut quotient, rem) = numerator.polynomial_division(&denominator);
        debug_assert!(rem.is_zero());

        quotient.pad((quotient.degree() + 1).next_power_of_two());
        quotient
    }
}

pub struct OpeningProof<F: Field> {
    fri_proof: FriProof<F>,
    // TODO: Get the degree from `CommonCircuitData` instead.
    quotient_degree: usize,
}

impl<F: Field> OpeningProof<F> {
    pub fn verify(
        &self,
        points: &[F],
        evaluations: &[Vec<F>],
        merkle_roots: &[Hash<F>],
        challenger: &mut Challenger<F>,
        fri_config: &FriConfig,
    ) -> Result<()> {
        for evals in evaluations {
            challenger.observe_elements(evals);
        }

        let alpha = challenger.get_challenge();

        let scaled_evals = evaluations
            .iter()
            .map(|e| reduce_with_powers(e, alpha))
            .collect::<Vec<_>>();

        let pairs = points
            .iter()
            .zip(&scaled_evals)
            .map(|(&x, &e)| (x, e))
            .collect::<Vec<_>>();

        verify_fri_proof(
            log2_strict(self.quotient_degree),
            &pairs,
            alpha,
            merkle_roots,
            &self.fri_proof,
            challenger,
            fri_config,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use anyhow::Result;

    fn gen_random_test_case<F: Field>(
        k: usize,
        degree_log: usize,
        num_points: usize,
    ) -> (Vec<PolynomialCoeffs<F>>, Vec<F>) {
        let degree = 1 << degree_log;

        let polys = (0..k)
            .map(|_| PolynomialCoeffs::new(F::rand_vec(degree)))
            .collect();
        let mut points = F::rand_vec(num_points);
        while points.iter().any(|&x| x.exp_usize(degree).is_one()) {
            points = F::rand_vec(num_points);
        }

        (polys, points)
    }

    #[test]
    fn test_polynomial_commitment() -> Result<()> {
        type F = CrandallField;

        let k = 10;
        let degree_log = 11;
        let num_points = 3;
        let fri_config = FriConfig {
            proof_of_work_bits: 2,
            rate_bits: 2,
            reduction_arity_bits: vec![3, 2, 1, 2],
            num_query_rounds: 3,
            blinding: false,
        };
        let (polys, points) = gen_random_test_case::<F>(k, degree_log, num_points);

        let lpc = ListPolynomialCommitment::new(polys, &fri_config);
        let (proof, evaluations) = lpc.open(&points, &mut Challenger::new());
        proof.verify(
            &points,
            &evaluations,
            &[lpc.merkle_tree.root],
            &mut Challenger::new(),
            &fri_config,
        )
    }

    #[test]
    fn test_polynomial_commitment_blinding() -> Result<()> {
        type F = CrandallField;

        let k = 10;
        let degree_log = 11;
        let num_points = 3;
        let fri_config = FriConfig {
            proof_of_work_bits: 2,
            rate_bits: 2,
            reduction_arity_bits: vec![3, 2, 1, 2],
            num_query_rounds: 3,
            blinding: true,
        };
        let (polys, points) = gen_random_test_case::<F>(k, degree_log, num_points);

        let lpc = ListPolynomialCommitment::new(polys, &fri_config);
        let (proof, evaluations) = lpc.open(&points, &mut Challenger::new());
        proof.verify(
            &points,
            &evaluations,
            &[lpc.merkle_tree.root],
            &mut Challenger::new(),
            &fri_config,
        )
    }

    #[test]
    fn test_batch_polynomial_commitment() -> Result<()> {
        type F = CrandallField;

        let k0 = 10;
        let k1 = 3;
        let k2 = 7;
        let degree_log = 11;
        let num_points = 5;
        let fri_config = FriConfig {
            proof_of_work_bits: 2,
            rate_bits: 2,
            reduction_arity_bits: vec![2, 3, 1, 2],
            num_query_rounds: 3,
            blinding: false,
        };
        let (polys0, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys1, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys2, points) = gen_random_test_case::<F>(k0, degree_log, num_points);

        let lpc0 = ListPolynomialCommitment::new(polys0, &fri_config);
        let lpc1 = ListPolynomialCommitment::new(polys1, &fri_config);
        let lpc2 = ListPolynomialCommitment::new(polys2, &fri_config);

        let (proof, evaluations) = ListPolynomialCommitment::batch_open(
            &[&lpc0, &lpc1, &lpc2],
            &points,
            &mut Challenger::new(),
        );
        proof.verify(
            &points,
            &evaluations,
            &[
                lpc0.merkle_tree.root,
                lpc1.merkle_tree.root,
                lpc2.merkle_tree.root,
            ],
            &mut Challenger::new(),
            &fri_config,
        )
    }

    #[test]
    fn test_batch_polynomial_commitment_blinding() -> Result<()> {
        type F = CrandallField;

        let k0 = 10;
        let k1 = 3;
        let k2 = 7;
        let degree_log = 11;
        let num_points = 5;
        let fri_config = FriConfig {
            proof_of_work_bits: 2,
            rate_bits: 2,
            reduction_arity_bits: vec![2, 3, 1, 2],
            num_query_rounds: 3,
            blinding: true,
        };
        let (polys0, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys1, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys2, points) = gen_random_test_case::<F>(k0, degree_log, num_points);

        let lpc0 = ListPolynomialCommitment::new(polys0, &fri_config);
        let lpc1 = ListPolynomialCommitment::new(polys1, &fri_config);
        let lpc2 = ListPolynomialCommitment::new(polys2, &fri_config);

        let (proof, evaluations) = ListPolynomialCommitment::batch_open(
            &[&lpc0, &lpc1, &lpc2],
            &points,
            &mut Challenger::new(),
        );
        proof.verify(
            &points,
            &evaluations,
            &[
                lpc0.merkle_tree.root,
                lpc1.merkle_tree.root,
                lpc2.merkle_tree.root,
            ],
            &mut Challenger::new(),
            &fri_config,
        )
    }
}
