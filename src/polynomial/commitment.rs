use crate::field::field::Field;
use crate::field::lagrange::interpolant;
use crate::fri::{fri_proof, verify_fri_proof, FriConfig};
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::reduce_with_powers;
use crate::polynomial::old_polynomial::Polynomial;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::proof::{FriProof, Hash};
use crate::util::{log2_strict, reverse_index_bits_in_place, transpose};
use anyhow::Result;

struct ListPolynomialCommitment<F: Field> {
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub fri_config: FriConfig,
    pub merkle_tree: MerkleTree<F>,
    pub degree: usize,
    pub blinding: bool,
}

impl<F: Field> ListPolynomialCommitment<F> {
    pub fn new(
        polynomials: Vec<PolynomialCoeffs<F>>,
        fri_config: &FriConfig,
        blinding: bool,
    ) -> Self {
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
            .chain(blinding.then(|| {
                (0..(degree << fri_config.rate_bits))
                    .map(|_| F::rand())
                    .collect()
            }))
            .collect::<Vec<_>>();

        let mut leaves = transpose(&lde_values);
        reverse_index_bits_in_place(&mut leaves);
        // let merkle_tree = MerkleTree::new(transpose(&lde_values), false);
        let merkle_tree = MerkleTree::new(leaves, false);

        Self {
            polynomials,
            fri_config: fri_config.clone(),
            merkle_tree,
            degree,
            blinding,
        }
    }

    pub fn open(&self, points: &[F], challenger: &mut Challenger<F>) -> OpeningProof<F> {
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

        challenger.observe_hash(&self.merkle_tree.root);
        let alpha = challenger.get_challenge();

        let scaled_poly = self
            .polynomials
            .iter()
            .rev()
            .map(|p| p.clone().into())
            .fold(Polynomial::empty(), |acc, p| acc.scalar_mul(alpha).add(&p));
        let scaled_evals = evaluations
            .iter()
            .map(|e| reduce_with_powers(e, alpha))
            .collect::<Vec<_>>();

        let pairs = points
            .iter()
            .zip(&scaled_evals)
            .map(|(&x, &e)| (x, e))
            .collect::<Vec<_>>();
        debug_assert!(pairs.iter().all(|&(x, e)| scaled_poly.eval(x) == e));

        let interpolant: Polynomial<F> = interpolant(&pairs).into();
        let denominator = points
            .iter()
            .fold(Polynomial::from(vec![F::ONE]), |acc, &x| {
                acc.mul(&vec![-x, F::ONE].into())
            });
        let numerator = scaled_poly.add(&interpolant.neg());
        let (mut quotient, rem) = numerator.polynomial_division(&denominator);
        debug_assert!(rem.is_zero());

        quotient.pad((quotient.degree() + 1).next_power_of_two());
        let lde_quotient = PolynomialCoeffs::from(quotient.clone()).lde(self.fri_config.rate_bits);
        let lde_quotient_values = lde_quotient
            .clone()
            .coset_fft(F::MULTIPLICATIVE_GROUP_GENERATOR);

        let fri_proof = fri_proof(
            &[self.merkle_tree.clone()],
            &lde_quotient,
            &lde_quotient_values,
            challenger,
            &self.fri_config,
        );

        OpeningProof {
            evaluations,
            merkle_root: self.merkle_tree.root,
            fri_proof,
            quotient_degree: quotient.len(),
        }
    }
}

pub struct OpeningProof<F: Field> {
    evaluations: Vec<Vec<F>>,
    merkle_root: Hash<F>,
    fri_proof: FriProof<F>,
    quotient_degree: usize,
}

impl<F: Field> OpeningProof<F> {
    pub fn verify(
        &self,
        points: &[F],
        challenger: &mut Challenger<F>,
        fri_config: &FriConfig,
    ) -> Result<()> {
        for evals in &self.evaluations {
            challenger.observe_elements(evals);
        }

        challenger.observe_hash(&self.merkle_root);
        let alpha = challenger.get_challenge();

        let scaled_evals = self
            .evaluations
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
            &[self.merkle_root],
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

    #[test]
    fn test_polynomial_commitment() -> Result<()> {
        type F = CrandallField;

        let k = 10;
        let degree_log = 11;
        let degree = 1 << degree_log;

        let fri_config = FriConfig {
            proof_of_work_bits: 2,
            rate_bits: 2,
            reduction_arity_bits: vec![3, 2, 1, 2],
            num_query_rounds: 3,
        };

        let polys = (0..k)
            .map(|_| PolynomialCoeffs::new((0..degree).map(|_| F::rand()).collect()))
            .collect();

        let lpc = ListPolynomialCommitment::new(polys, &fri_config, false);

        let num_points = 3;
        let points = (0..num_points).map(|_| F::rand()).collect::<Vec<_>>();

        let proof = lpc.open(&points, &mut Challenger::new());

        proof.verify(&points, &mut Challenger::new(), &fri_config)
    }
}
