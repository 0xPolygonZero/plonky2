use plonky2::field::extension_field::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::fri::prover::fri_proof;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;
use plonky2_util::log2_strict;
use rayon::prelude::*;

use crate::config::StarkConfig;
use crate::proof::StarkProof;
use crate::stark::Stark;

/// Returns a witness matrix in row-major form.
fn generate_witness<F, S, const D: usize>(stark: S) -> Vec<Vec<F>>
where
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
{
    let mut row = stark.generate_first_row();
    todo!()
}

pub fn prove<F, C, S, const D: usize>(
    stark: S,
    config: StarkConfig,
    timing: &mut TimingTree,
) -> StarkProof<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
{
    let witness_row_major: Vec<Vec<F>> = timed!(
        timing,
        "generate witness",
        generate_witness::<F, S, D>(stark)
    );
    let degree_bits = log2_strict(witness_row_major.len());
    let witness_col_major: Vec<Vec<F>> = transpose(&witness_row_major);

    let trace_poly_values: Vec<PolynomialValues<F>> = timed!(
        timing,
        "compute trace polynomials",
        witness_col_major
            .par_iter()
            .map(|column| PolynomialValues::new(column.clone()))
            .collect()
    );

    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    let trace_commitment = timed!(
        timing,
        "compute trace commitment",
        PolynomialBatch::<F, C, D>::from_values(
            trace_poly_values,
            rate_bits,
            false,
            cap_height,
            timing,
            None,
        )
    );

    let trace_cap = trace_commitment.merkle_tree.cap;
    let openings = todo!();

    let initial_merkle_trees = todo!();
    let lde_polynomial_coeffs = todo!();
    let lde_polynomial_values = todo!();
    let mut challenger = Challenger::new();
    let fri_params = config.fri_params(degree_bits);

    let opening_proof = fri_proof::<F, C, D>(
        initial_merkle_trees,
        lde_polynomial_coeffs,
        lde_polynomial_values,
        &mut challenger,
        &fri_params,
        timing,
    );

    StarkProof {
        trace_cap,
        openings,
        opening_proof,
    }
}
