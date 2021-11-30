use rayon::prelude::*;

use crate::field::extension_field::Extendable;
use crate::field::fft::FftRootTable;
use crate::field::field_types::{Field, RichField};
use crate::fri::proof::FriProof;
use crate::fri::prover::fri_proof;
use crate::hash::merkle_tree::MerkleTree;
use crate::iop::challenger::Challenger;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::plonk::proof::OpeningSet;
use crate::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::timed;
use crate::util::reducing::ReducingFactor;
use crate::util::timing::TimingTree;
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place, transpose};

/// Four (~64 bit) field elements gives ~128 bit security.
pub const SALT_SIZE: usize = 4;

/// Represents a batch FRI based commitment to a list of polynomials.
pub struct PolynomialBatchCommitment<F: RichField> {
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub merkle_tree: MerkleTree<F>,
    pub degree_log: usize,
    pub rate_bits: usize,
    pub blinding: bool,
}

impl<F: RichField> PolynomialBatchCommitment<F> {
    /// Creates a list polynomial commitment for the polynomials interpolating the values in `values`.
    pub(crate) fn from_values(
        values: Vec<PolynomialValues<F>>,
        rate_bits: usize,
        blinding: bool,
        cap_height: usize,
        timing: &mut TimingTree,
        fft_root_table: Option<&FftRootTable<F>>,
    ) -> Self {
        let coeffs = timed!(
            timing,
            "IFFT",
            values.par_iter().map(|v| v.ifft()).collect::<Vec<_>>()
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
    pub(crate) fn from_coeffs(
        polynomials: Vec<PolynomialCoeffs<F>>,
        rate_bits: usize,
        blinding: bool,
        cap_height: usize,
        timing: &mut TimingTree,
        fft_root_table: Option<&FftRootTable<F>>,
    ) -> Self {
        let degree = polynomials[0].len();
        let lde_values = timed!(
            timing,
            "FFT + blinding",
            Self::lde_values(&polynomials, rate_bits, blinding, fft_root_table)
        );

        let mut leaves = timed!(timing, "transpose LDEs", transpose(&lde_values));
        reverse_index_bits_in_place(&mut leaves);
        let merkle_tree = timed!(
            timing,
            "build Merkle tree",
            MerkleTree::new(leaves, cap_height)
        );

        Self {
            polynomials,
            merkle_tree,
            degree_log: log2_strict(degree),
            rate_bits,
            blinding,
        }
    }

    fn lde_values(
        polynomials: &[PolynomialCoeffs<F>],
        rate_bits: usize,
        blinding: bool,
        fft_root_table: Option<&FftRootTable<F>>,
    ) -> Vec<Vec<F>> {
        let degree = polynomials[0].len();

        // If blinding, salt with two random elements to each leaf vector.
        let salt_size = if blinding { SALT_SIZE } else { 0 };

        polynomials
            .par_iter()
            .map(|p| {
                assert_eq!(p.len(), degree, "Polynomial degrees inconsistent");
                p.lde(rate_bits)
                    .coset_fft_with_options(F::coset_shift(), Some(rate_bits), fft_root_table)
                    .values
            })
            .chain(
                (0..salt_size)
                    .into_par_iter()
                    .map(|_| F::rand_vec(degree << rate_bits)),
            )
            .collect()
    }

    pub fn get_lde_values(&self, index: usize) -> &[F] {
        let index = reverse_bits(index, self.degree_log + self.rate_bits);
        let slice = &self.merkle_tree.leaves[index];
        &slice[..slice.len() - if self.blinding { SALT_SIZE } else { 0 }]
    }

    /// Takes the commitments to the constants - sigmas - wires - zs - quotient — polynomials,
    /// and an opening point `zeta` and produces a batched opening proof + opening set.
    pub(crate) fn open_plonk<const D: usize>(
        commitments: &[&Self; 4],
        zeta: F::Extension,
        challenger: &mut Challenger<F>,
        common_data: &CommonCircuitData<F, D>,
        timing: &mut TimingTree,
    ) -> (FriProof<F, D>, OpeningSet<F, D>)
    where
        F: RichField + Extendable<D>,
    {
        let config = &common_data.config;
        assert!(D > 1, "Not implemented for D=1.");
        let degree_log = commitments[0].degree_log;
        let g = F::Extension::primitive_root_of_unity(degree_log);
        for p in &[zeta, g * zeta] {
            assert_ne!(
                p.exp_u64(1 << degree_log as u64),
                F::Extension::ONE,
                "Opening point is in the subgroup."
            );
        }

        let os = timed!(
            timing,
            "construct the opening set",
            OpeningSet::new(
                zeta,
                g,
                commitments[0],
                commitments[1],
                commitments[2],
                commitments[3],
                common_data,
            )
        );
        challenger.observe_opening_set(&os);

        let alpha = challenger.get_extension_challenge();
        let mut alpha = ReducingFactor::new(alpha);

        // Final low-degree polynomial that goes into FRI.
        let mut final_poly = PolynomialCoeffs::empty();

        let mut zs_polys = commitments[PlonkPolynomials::ZS_PARTIAL_PRODUCTS.index]
            .polynomials
            .iter()
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
        .chain(partial_products_polys);
        let single_composition_poly = timed!(
            timing,
            "reduce single polys",
            alpha.reduce_polys_base(single_polys)
        );

        let single_quotient = Self::compute_quotient([zeta], single_composition_poly);
        final_poly += single_quotient;
        alpha.reset();

        // Zs polynomials are opened at `zeta` and `g*zeta`.
        let zs_composition_poly = timed!(
            timing,
            "reduce Z polys",
            alpha.reduce_polys_base(zs_polys.into_iter())
        );

        let zs_quotient = Self::compute_quotient([zeta, g * zeta], zs_composition_poly);
        alpha.shift_poly(&mut final_poly);
        final_poly += zs_quotient;

        let lde_final_poly = final_poly.lde(config.rate_bits);
        let lde_final_values = timed!(
            timing,
            &format!("perform final FFT {}", lde_final_poly.len()),
            lde_final_poly.coset_fft(F::coset_shift().into())
        );

        let fri_proof = fri_proof(
            &commitments
                .par_iter()
                .map(|c| &c.merkle_tree)
                .collect::<Vec<_>>(),
            lde_final_poly,
            lde_final_values,
            challenger,
            common_data,
            timing,
        );

        (fri_proof, os)
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
