use crate::field::fft::fft;
use crate::field::field::Field;
use crate::field::lagrange::{interpolant, interpolate};
use crate::fri::{fri_proof, FriConfig};
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::reduce_with_powers;
use crate::polynomial::old_polynomial::Polynomial;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::util::transpose;

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
        let mut lde_values = polynomials
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

        let merkle_tree = MerkleTree::new(transpose(&lde_values), false);

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
        let denominator = points.iter().fold(Polynomial::empty(), |acc, &x| {
            acc.mul(&vec![-x, F::ONE].into())
        });
        let numerator = scaled_poly.add(&interpolant.neg());
        let (mut quotient, rem) = numerator.polynomial_division(&denominator);
        debug_assert!(rem.is_zero());
        quotient.pad(quotient.degree().next_power_of_two());
        let quotient_values = fft(quotient.clone().into());
        let fri_proof = fri_proof(
            &quotient.into(),
            &quotient_values,
            challenger,
            &self.fri_config,
        );
        todo!()
    }
}

pub struct OpeningProof<F: Field> {
    evaluations: Vec<Vec<F>>,
}
