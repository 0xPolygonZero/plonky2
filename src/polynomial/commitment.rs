use anyhow::Result;
use rayon::prelude::*;

use crate::field::extension_field::Extendable;
use crate::field::extension_field::FieldExtension;
use crate::field::field::Field;
use crate::field::lagrange::interpolant;
use crate::fri::{prover::fri_proof, verifier::verify_fri_proof, FriConfig};
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::reduce_with_powers;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::proof::{FriProof, Hash, OpeningSet};
use crate::timed;
use crate::util::{log2_strict, reverse_index_bits_in_place, transpose};

pub const SALT_SIZE: usize = 2;
pub const EXTENSION_DEGREE: usize = 2;

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

    pub fn open(
        &self,
        points: &[F::Extension],
        challenger: &mut Challenger<F>,
        config: &FriConfig,
    ) -> (OpeningProof<F>, Vec<Vec<F::Extension>>)
    where
        F: Extendable<EXTENSION_DEGREE>,
    {
        assert_eq!(self.rate_bits, config.rate_bits);
        assert_eq!(config.blinding.len(), 1);
        assert_eq!(self.blinding, config.blinding[0]);
        for p in points {
            assert_ne!(
                p.exp_usize(self.degree),
                F::Extension::ONE,
                "Opening point is in the subgroup."
            );
        }

        let evaluations = points
            .par_iter()
            .map(|&x| {
                self.polynomials
                    .iter()
                    .map(|p| p.to_extension().eval(x))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for evals in &evaluations {
            for e in evals {
                challenger.observe_extension_element(e);
            }
        }

        let alpha = challenger.get_extension_challenge();

        // Scale polynomials by `alpha`.
        let composition_poly = self
            .polynomials
            .iter()
            .rev()
            .fold(PolynomialCoeffs::zero(self.degree), |acc, p| {
                &(&acc * alpha) + &p.to_extension()
            });
        // Scale evaluations by `alpha`.
        let composition_evals = evaluations
            .par_iter()
            .map(|e| reduce_with_powers(e, alpha))
            .collect::<Vec<_>>();

        let quotient = Self::compute_quotient(points, &composition_evals, &composition_poly);

        let lde_quotient = PolynomialCoeffs::from(quotient.clone()).lde(self.rate_bits);
        let lde_quotient_values = lde_quotient
            .clone()
            .coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR.into());

        let fri_proof = fri_proof(
            &[&self.merkle_tree],
            &lde_quotient,
            &lde_quotient_values,
            challenger,
            &config,
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
        points: &[F::Extension],
        challenger: &mut Challenger<F>,
        config: &FriConfig,
    ) -> (OpeningProof<F>, Vec<Vec<Vec<F::Extension>>>)
    where
        F: Extendable<EXTENSION_DEGREE>,
    {
        let degree = commitments[0].degree;
        assert_eq!(config.blinding.len(), commitments.len());
        for (i, commitment) in commitments.iter().enumerate() {
            assert_eq!(commitment.rate_bits, config.rate_bits, "Invalid rate.");
            assert_eq!(
                commitment.blinding, config.blinding[i],
                "Invalid blinding paramater."
            );
            assert_eq!(
                commitment.degree, degree,
                "Trying to open polynomial commitments of different degrees."
            );
        }
        for p in points {
            assert_ne!(
                p.exp_usize(degree),
                F::Extension::ONE,
                "Opening point is in the subgroup."
            );
        }

        let evaluations = points
            .par_iter()
            .map(|&x| {
                commitments
                    .iter()
                    .map(move |c| {
                        c.polynomials
                            .iter()
                            .map(|p| p.to_extension().eval(x))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for evals_per_point in &evaluations {
            for evals in evals_per_point {
                challenger.observe_extension_elements(evals);
            }
        }

        let alpha = challenger.get_extension_challenge();

        // Scale polynomials by `alpha`.
        let composition_poly = commitments
            .iter()
            .flat_map(|c| &c.polynomials)
            .rev()
            .fold(PolynomialCoeffs::zero(degree), |acc, p| {
                &(&acc * alpha) + &p.to_extension()
            });
        // Scale evaluations by `alpha`.
        let composition_evals = &evaluations
            .par_iter()
            .map(|v| {
                v.iter()
                    .flatten()
                    .rev()
                    .fold(F::Extension::ZERO, |acc, &e| acc * alpha + e)
            })
            .collect::<Vec<_>>();

        let quotient = Self::compute_quotient(points, &composition_evals, &composition_poly);

        let lde_quotient = PolynomialCoeffs::from(quotient.clone()).lde(config.rate_bits);
        let lde_quotient_values = lde_quotient.clone().coset_fft(F::Extension::from_basefield(
            F::MULTIPLICATIVE_GROUP_GENERATOR,
        ));

        let fri_proof = fri_proof(
            &commitments
                .par_iter()
                .map(|c| &c.merkle_tree)
                .collect::<Vec<_>>(),
            &lde_quotient,
            &lde_quotient_values,
            challenger,
            &config,
        );

        (
            OpeningProof {
                fri_proof,
                quotient_degree: quotient.len(),
            },
            evaluations,
        )
    }

    pub fn batch_open_plonk(
        commitments: &[&Self; 5],
        points: &[F::Extension],
        challenger: &mut Challenger<F>,
        config: &FriConfig,
    ) -> (OpeningProof<F>, Vec<OpeningSet<F::Extension>>)
    where
        F: Extendable<EXTENSION_DEGREE>,
    {
        let (op, mut evaluations) = Self::batch_open(commitments, points, challenger, config);
        let opening_sets = evaluations
            .par_iter_mut()
            .map(|evals| {
                evals.reverse();
                OpeningSet {
                    constants: evals.pop().unwrap(),
                    plonk_sigmas: evals.pop().unwrap(),
                    wires: evals.pop().unwrap(),
                    plonk_zs: evals.pop().unwrap(),
                    quotient_polys: evals.pop().unwrap(),
                }
            })
            .collect();
        (op, opening_sets)
    }

    /// Given `points=(x_i)`, `evals=(y_i)` and `poly=P` with `P(x_i)=y_i`, computes the polynomial
    /// `Q=(P-I)/Z` where `I` interpolates `(x_i, y_i)` and `Z` is the vanishing polynomial on `(x_i)`.
    fn compute_quotient(
        points: &[F::Extension],
        evals: &[F::Extension],
        poly: &PolynomialCoeffs<F::Extension>,
    ) -> PolynomialCoeffs<F::Extension>
    where
        F: Extendable<EXTENSION_DEGREE>,
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
        let (mut quotient, rem) = numerator.div_rem(&denominator);
        debug_assert!(rem.is_zero());

        quotient.padded(quotient.degree_plus_one().next_power_of_two())
    }
}

pub struct OpeningProof<F: Field + Extendable<EXTENSION_DEGREE>> {
    fri_proof: FriProof<F>,
    // TODO: Get the degree from `CommonCircuitData` instead.
    quotient_degree: usize,
}

impl<F: Field + Extendable<EXTENSION_DEGREE>> OpeningProof<F> {
    pub fn verify(
        &self,
        points: &[F::Extension],
        evaluations: &[Vec<Vec<F::Extension>>],
        merkle_roots: &[Hash<F>],
        challenger: &mut Challenger<F>,
        fri_config: &FriConfig,
    ) -> Result<()> {
        for evals_per_point in evaluations {
            for evals in evals_per_point {
                challenger.observe_extension_elements(evals);
            }
        }

        let alpha = challenger.get_extension_challenge();

        let scaled_evals = evaluations
            .par_iter()
            .map(|v| {
                v.iter()
                    .flatten()
                    .rev()
                    .fold(F::Extension::ZERO, |acc, &e| acc * alpha + e)
            })
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
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;

    use super::*;

    fn gen_random_test_case<F: Field + Extendable<EXTENSION_DEGREE>>(
        k: usize,
        degree_log: usize,
        num_points: usize,
    ) -> (Vec<PolynomialCoeffs<F>>, Vec<F::Extension>) {
        let degree = 1 << degree_log;

        let polys = (0..k)
            .map(|_| PolynomialCoeffs::new(F::rand_vec(degree)))
            .collect();
        let mut points = F::Extension::rand_vec(num_points);
        while points.iter().any(|&x| x.exp_usize(degree).is_one()) {
            points = F::Extension::rand_vec(num_points);
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
            blinding: vec![false],
        };
        let (polys, points) = gen_random_test_case::<F>(k, degree_log, num_points);

        let lpc = ListPolynomialCommitment::new(polys, fri_config.rate_bits, false);
        let (proof, evaluations) = lpc.open(&points, &mut Challenger::new(), &fri_config);
        proof.verify(
            &points,
            &evaluations.into_iter().map(|e| vec![e]).collect::<Vec<_>>(),
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
            blinding: vec![true],
        };
        let (polys, points) = gen_random_test_case::<F>(k, degree_log, num_points);

        let lpc = ListPolynomialCommitment::new(polys, fri_config.rate_bits, true);
        let (proof, evaluations) = lpc.open(&points, &mut Challenger::new(), &fri_config);
        proof.verify(
            &points,
            &evaluations.into_iter().map(|e| vec![e]).collect::<Vec<_>>(),
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
            blinding: vec![false, false, false],
        };
        let (polys0, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys1, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys2, points) = gen_random_test_case::<F>(k0, degree_log, num_points);

        let lpc0 = ListPolynomialCommitment::new(polys0, fri_config.rate_bits, false);
        let lpc1 = ListPolynomialCommitment::new(polys1, fri_config.rate_bits, false);
        let lpc2 = ListPolynomialCommitment::new(polys2, fri_config.rate_bits, false);

        let (proof, evaluations) = ListPolynomialCommitment::batch_open(
            &[&lpc0, &lpc1, &lpc2],
            &points,
            &mut Challenger::new(),
            &fri_config,
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
            blinding: vec![true, false, true],
        };
        let (polys0, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys1, _) = gen_random_test_case::<F>(k0, degree_log, num_points);
        let (polys2, points) = gen_random_test_case::<F>(k0, degree_log, num_points);

        let lpc0 = ListPolynomialCommitment::new(polys0, fri_config.rate_bits, true);
        let lpc1 = ListPolynomialCommitment::new(polys1, fri_config.rate_bits, false);
        let lpc2 = ListPolynomialCommitment::new(polys2, fri_config.rate_bits, true);

        let (proof, evaluations) = ListPolynomialCommitment::batch_open(
            &[&lpc0, &lpc1, &lpc2],
            &points,
            &mut Challenger::new(),
            &fri_config,
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
