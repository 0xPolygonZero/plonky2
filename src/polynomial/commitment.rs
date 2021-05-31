use anyhow::Result;
use rayon::prelude::*;

use crate::field::extension_field::FieldExtension;
use crate::field::extension_field::{Extendable, OEF};
use crate::field::field::Field;
use crate::field::lagrange::interpolant;
use crate::fri::{prover::fri_proof, verifier::verify_fri_proof, FriConfig};
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::{reduce_polys_with_powers, reduce_with_powers};
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::proof::{FriInitialTreeProof, FriProof, Hash, OpeningSet};
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

    pub fn open<const D: usize>(
        &self,
        points: &[F::Extension],
        challenger: &mut Challenger<F>,
        config: &FriConfig,
    ) -> (OpeningProof<F, D>, Vec<Vec<F::Extension>>)
    where
        F: Extendable<D>,
    {
        assert_eq!(self.rate_bits, config.rate_bits);
        assert_eq!(config.check_basefield.len(), 1);
        assert_eq!(config.blinding.len(), 1);
        assert_eq!(self.blinding, config.blinding[0]);
        for p in points {
            assert_ne!(
                p.exp(self.degree as u64),
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
        let composition_poly = reduce_polys_with_powers(&self.polynomials, alpha);
        // Scale evaluations by `alpha`.
        let composition_evals = evaluations
            .par_iter()
            .map(|e| reduce_with_powers(e, alpha))
            .collect::<Vec<_>>();

        let quotient = Self::compute_quotient(points, &composition_evals, &composition_poly);

        let quotient = if config.check_basefield[0] {
            let composition_poly_conj = PolynomialCoeffs::<F>::frobenius(&composition_poly);
            // This equality holds iff the polynomials in `self.polynomials` are defined over `F` and not `F::Extension`.
            debug_assert_eq!(
                composition_poly_conj.eval(points[0].frobenius()),
                composition_evals[0].frobenius()
            );
            let quotient_conj = Self::compute_quotient(
                &[points[0].frobenius()],
                &[composition_evals[0].frobenius()],
                &composition_poly_conj,
            );

            &(&quotient_conj * alpha.exp(self.polynomials.len() as u64)) + &quotient
        } else {
            quotient
        };

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

    // pub fn batch_open<const D: usize>(
    //     commitments: &[&Self],
    //     opening_config: &OpeningConfig<F, D>,
    //     fri_config: &FriConfig,
    //     challenger: &mut Challenger<F>,
    // ) -> (OpeningProof<F, D>, Vec<Vec<Vec<Vec<F::Extension>>>>)
    // where
    //     F: Extendable<D>,
    // {
    //     let degree = commitments[0].degree;
    //     assert_eq!(fri_config.blinding.len(), commitments.len());
    //     for (i, commitment) in commitments.iter().enumerate() {
    //         assert_eq!(commitment.rate_bits, fri_config.rate_bits, "Invalid rate.");
    //         assert_eq!(
    //             commitment.blinding, fri_config.blinding[i],
    //             "Invalid blinding paramater."
    //         );
    //         assert_eq!(
    //             commitment.degree, degree,
    //             "Trying to open polynomial commitments of different degrees."
    //         );
    //     }
    //     for &p in opening_config.points.iter().flat_map(|(v, _)| v) {
    //         assert_ne!(
    //             p.exp(degree as u64),
    //             F::Extension::ONE,
    //             "Opening point is in the subgroup."
    //         );
    //     }
    //
    //     let evaluations = opening_config
    //         .points
    //         .iter()
    //         .map(|(xs, is)| {
    //             xs.iter()
    //                 .map(|&x| {
    //                     is.iter()
    //                         .map(|&i| {
    //                             commitments[i]
    //                                 .polynomials
    //                                 .iter()
    //                                 .map(|p| p.to_extension().eval(x))
    //                                 .collect::<Vec<_>>()
    //                         })
    //                         .collect::<Vec<_>>()
    //                 })
    //                 .collect::<Vec<_>>()
    //         })
    //         .collect::<Vec<_>>();
    //     for evals_per_point_vec in &evaluations {
    //         for evals_per_point in evals_per_point_vec {
    //             for evals in evals_per_point {
    //                 challenger.observe_extension_elements(evals);
    //             }
    //         }
    //     }
    //
    //     let alpha = challenger.get_extension_challenge();
    //     let mut cur_alpha = F::Extension::ONE;
    //
    //     // Final low-degree polynomial that goes into FRI.
    //     let mut final_poly = PolynomialCoeffs::empty();
    //
    //     for ((ps, is), evals) in opening_config.points.iter().zip(&evaluations) {
    //         let mut poly_count = 0;
    //         // Scale polynomials by `alpha`.
    //         let composition_poly = is
    //             .iter()
    //             .flat_map(|&i| &commitments[i].polynomials)
    //             .rev()
    //             .fold(PolynomialCoeffs::zero(degree), |acc, p| {
    //                 poly_count += 1;
    //                 &(&acc * alpha) + &p.to_extension()
    //             });
    //         // Scale evaluations by `alpha`.
    //         let composition_evals = &evals
    //             .iter()
    //             .map(|v| {
    //                 v.iter()
    //                     .flatten()
    //                     .rev()
    //                     .fold(F::Extension::ZERO, |acc, &e| acc * alpha + e)
    //             })
    //             .collect::<Vec<_>>();
    //
    //         let quotient = Self::compute_quotient(ps, &composition_evals, &composition_poly);
    //         final_poly = &final_poly + &(&quotient * cur_alpha);
    //         cur_alpha *= alpha.exp(poly_count);
    //     }
    //
    //     for &i in &opening_config.check_base_field {
    //         let commitment = commitments[i];
    //         let x = opening_config
    //             .points
    //             .iter()
    //             .find(|(xs, is)| is.contains(&i))
    //             .expect("Polynomial is never opened.")
    //             .0[0];
    //         let x_conj = x.frobenius();
    //         let mut poly_count = 0;
    //         let poly = commitment.polynomials.iter().rev().fold(
    //             PolynomialCoeffs::zero(degree),
    //             |acc, p| {
    //                 poly_count += 1;
    //                 &(&acc * alpha) + &p.to_extension()
    //             },
    //         );
    //         let e = poly.eval(x_conj);
    //         let quotient = Self::compute_quotient(&[x_conj], &[e], &poly);
    //         final_poly = &final_poly + &(&quotient * cur_alpha);
    //         cur_alpha *= alpha.exp(poly_count);
    //     }
    //
    //     let lde_final_poly = final_poly.lde(fri_config.rate_bits);
    //     let lde_final_values = lde_final_poly
    //         .clone()
    //         .coset_fft(F::Extension::from_basefield(
    //             F::MULTIPLICATIVE_GROUP_GENERATOR,
    //         ));
    //
    //     let fri_proof = fri_proof(
    //         &commitments
    //             .par_iter()
    //             .map(|c| &c.merkle_tree)
    //             .collect::<Vec<_>>(),
    //         &lde_final_poly,
    //         &lde_final_values,
    //         challenger,
    //         &fri_config,
    //     );
    //
    //     (
    //         OpeningProof {
    //             fri_proof,
    //             quotient_degree: final_poly.len(),
    //         },
    //         evaluations,
    //     )
    // }

    pub fn open_plonk<const D: usize>(
        commitments: &[&Self; 5],
        zeta: F::Extension,
        degree_log: usize,
        challenger: &mut Challenger<F>,
        config: &FriConfig,
    ) -> (OpeningProof<F, D>, OpeningSet<F, D>)
    where
        F: Extendable<D>,
    {
        let g = F::Extension::primitive_root_of_unity(degree_log);
        dbg!(degree_log);
        for &p in &[zeta, g * zeta] {
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
        dbg!(alpha);
        let mut cur_alpha = F::Extension::ONE;

        // Final low-degree polynomial that goes into FRI.
        let mut final_poly = PolynomialCoeffs::empty();
        // Count the total number of polynomials accumulated into `final_poly`.
        let mut poly_count = 0;

        let composition_poly = [0, 1, 4]
            .iter()
            .flat_map(|&i| &commitments[i].polynomials)
            .rev()
            .fold(PolynomialCoeffs::empty(), |acc, p| {
                poly_count += 1;
                &(&acc * alpha) + &p.to_extension()
            });
        let composition_eval = [&os.constants, &os.plonk_sigmas, &os.quotient_polys]
            .iter()
            .flat_map(|v| v.iter())
            .rev()
            .fold(F::Extension::ZERO, |acc, &e| acc * alpha + e);

        let quotient = Self::compute_quotient(&[zeta], &[composition_eval], &composition_poly);
        final_poly = &final_poly + &(&quotient * cur_alpha);
        {
            let lde_final_poly = final_poly.lde(config.rate_bits);
            let lde_final_values = lde_final_poly
                .clone()
                .coset_fft(F::Extension::from_basefield(
                    F::MULTIPLICATIVE_GROUP_GENERATOR,
                ));
            dbg!(lde_final_values);
        }
        cur_alpha = alpha.exp(poly_count);

        let zs_composition_poly =
            commitments[3]
                .polynomials
                .iter()
                .rev()
                .fold(PolynomialCoeffs::empty(), |acc, p| {
                    poly_count += 1;
                    &(&acc * alpha) + &p.to_extension()
                });
        let zs_composition_evals = [
            reduce_with_powers(&os.plonk_zs, alpha),
            reduce_with_powers(&os.plonk_zs_right, alpha),
        ];

        let zs_quotient = Self::compute_quotient(
            &[zeta, g * zeta],
            &zs_composition_evals,
            &zs_composition_poly,
        );
        final_poly = &final_poly + &(&zs_quotient * cur_alpha);
        {
            let lde_final_poly = final_poly.lde(config.rate_bits);
            let lde_final_values = lde_final_poly
                .clone()
                .coset_fft(F::Extension::from_basefield(
                    F::MULTIPLICATIVE_GROUP_GENERATOR,
                ));
            dbg!(lde_final_values);
            dbg!(cur_alpha);
        }
        cur_alpha = alpha.exp(poly_count);

        if D > 1 {
            let wires_composition_poly = commitments[2].polynomials.iter().rev().fold(
                PolynomialCoeffs::empty(),
                |acc, p| {
                    poly_count += 1;
                    &(&acc * alpha) + &p.to_extension()
                },
            );
            let wire_evals_frob = os.wires.iter().map(|e| e.frobenius()).collect::<Vec<_>>();
            let wires_composition_evals = [
                reduce_with_powers(&os.wires, alpha),
                reduce_with_powers(&wire_evals_frob, alpha),
            ];

            let wires_quotient = Self::compute_quotient(
                &[zeta, zeta.frobenius()],
                &wires_composition_evals,
                &wires_composition_poly,
            );
            final_poly = &final_poly + &(&wires_quotient * cur_alpha);
        }

        dbg!(final_poly.coeffs.len());
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

        dbg!(&pairs);
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
        dbg!(alpha);

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

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;

    use super::*;
    use std::convert::TryInto;

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
            blinding: vec![false, false, false, false, false],
            check_basefield: vec![false, false, false],
        };

        let lpcs = ks
            .iter()
            .map(|&k| {
                ListPolynomialCommitment::<F>::new(
                    gen_random_test_case(k, degree_log),
                    fri_config.rate_bits,
                    false,
                )
            })
            .collect::<Vec<_>>();

        let zeta = gen_random_point::<F, D>(degree_log);
        let (proof, os) = ListPolynomialCommitment::open_plonk::<D>(
            &[&lpcs[0], &lpcs[1], &lpcs[2], &lpcs[3], &lpcs[4]],
            zeta,
            degree_log,
            &mut Challenger::new(),
            &fri_config,
        );
        let os = OpeningSet::new(
            zeta,
            F::Extension::primitive_root_of_unity(degree_log),
            &lpcs[0],
            &lpcs[1],
            &lpcs[2],
            &lpcs[3],
            &lpcs[4],
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

    // fn check_batch_polynomial_commitment_blinding<F: Field + Extendable<D>, const D: usize>(
    // ) -> Result<()> {
    //     let k0 = 10;
    //     let k1 = 3;
    //     let k2 = 7;
    //     let degree_log = 11;
    //     let num_points = 5;
    //     let fri_config = FriConfig {
    //         proof_of_work_bits: 2,
    //         rate_bits: 2,
    //         reduction_arity_bits: vec![2, 3, 1, 2],
    //         num_query_rounds: 3,
    //         blinding: vec![true, false, true],
    //         check_basefield: vec![true, false, true],
    //     };
    //     let (polys0, _) = gen_random_test_case::<F, D>(k0, degree_log, num_points);
    //     let (polys1, _) = gen_random_test_case::<F, D>(k1, degree_log, num_points);
    //     let (polys2, points) = gen_random_test_case::<F, D>(k2, degree_log, num_points);
    //
    //     let lpc0 = ListPolynomialCommitment::new(polys0, fri_config.rate_bits, true);
    //     let lpc1 = ListPolynomialCommitment::new(polys1, fri_config.rate_bits, false);
    //     let lpc2 = ListPolynomialCommitment::new(polys2, fri_config.rate_bits, true);
    //
    //     let (proof, evaluations) = ListPolynomialCommitment::batch_open::<D>(
    //         &[&lpc0, &lpc1, &lpc2],
    //         &points,
    //         &fri_config,
    //         &mut Challenger::new(),
    //     );
    //     proof.verify(
    //         &points,
    //         &evaluations,
    //         &[
    //             lpc0.merkle_tree.root,
    //             lpc1.merkle_tree.root,
    //             lpc2.merkle_tree.root,
    //         ],
    //         &mut Challenger::new(),
    //         &fri_config,
    //     )
    // }

    macro_rules! tests_commitments {
        ($F:ty, $D:expr) => {
            use super::*;

            #[test]
            fn test_batch_polynomial_commitment() -> Result<()> {
                check_batch_polynomial_commitment::<$F, $D>()
            }

            // #[test]
            // fn test_batch_polynomial_commitment_blinding() -> Result<()> {
            //     check_batch_polynomial_commitment_blinding::<$F, $D>()
            // }
        };
    }

    mod base {
        tests_commitments!(crate::field::crandall_field::CrandallField, 1);
    }

    mod quadratic {
        tests_commitments!(crate::field::crandall_field::CrandallField, 2);
    }

    mod quartic {
        use super::*;
        tests_commitments!(crate::field::crandall_field::CrandallField, 4);
    }
}
