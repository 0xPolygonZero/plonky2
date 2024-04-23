#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use itertools::Itertools;
use plonky2_field::extension::Extendable;
use plonky2_field::fft::FftRootTable;
use plonky2_field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2_maybe_rayon::*;
use plonky2_util::{log2_strict, reverse_index_bits_in_place};

use crate::fri::oracle::PolynomialBatch;
use crate::hash::field_merkle_tree::FieldMerkleTree;
use crate::hash::hash_types::RichField;
use crate::plonk::config::GenericConfig;
use crate::timed;
use crate::util::timing::TimingTree;
use crate::util::transpose;

/// Represents a batch FRI oracle, i.e. a batch of polynomials with different degrees which have
/// been Merkle-ized in a Field Merkle Tree.
#[derive(Eq, PartialEq, Debug)]
pub struct BatchFriOracle<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
{
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub field_merkle_tree: FieldMerkleTree<F, C::Hasher>,
    pub degree_logs: Vec<usize>,
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
        let degree_logs = polynomials
            .iter()
            .map(|p| log2_strict(p.len()))
            .collect_vec();
        assert!(degree_logs.windows(2).all(|pair| { pair[0] >= pair[1] }));

        let num_polynomials = polynomials.len();
        let mut group_start = 0;
        let mut leaves = Vec::new();

        for (i, d) in degree_logs.iter().enumerate() {
            if i == num_polynomials - 1 || *d > degree_logs[i + 1] {
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

        Self {
            polynomials,
            field_merkle_tree,
            degree_logs,
            rate_bits,
            blinding,
        }
    }
}
