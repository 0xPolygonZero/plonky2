use std::mem::swap;

use anyhow::Result;
use rayon::prelude::*;

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::fri::commitment::PolynomialBatchCommitment;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::hash_n_to_hash;
use crate::iop::challenger::Challenger;
use crate::iop::generator::generate_partial_witness;
use crate::iop::witness::{MatrixWitness, PartialWitness, Witness};
use crate::plonk::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::plonk::plonk_common::ZeroPolyOnCoset;
use crate::plonk::proof::{Proof, ProofWithPublicInputs};
use crate::plonk::vanishing_poly::eval_vanishing_poly_base_batch;
use crate::plonk::vars::EvaluationVarsBaseBatch;
use crate::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::timed;
use crate::util::partial_products::{partial_products_and_z_gx, quotient_chunk_products};
use crate::util::timing::TimingTree;
use crate::util::{log2_ceil, transpose};

pub(crate) fn prove<F: RichField + Extendable<D>, const D: usize>(
    prover_data: &ProverOnlyCircuitData<F, D>,
    common_data: &CommonCircuitData<F, D>,
    inputs: PartialWitness<F>,
    timing: &mut TimingTree,
) -> Result<ProofWithPublicInputs<F, D>> {
    let config = &common_data.config;
    let num_challenges = config.num_challenges;
    let quotient_degree = common_data.quotient_degree();
    let degree = common_data.degree();

    let partition_witness = timed!(
        timing,
        &format!("run {} generators", prover_data.generators.len()),
        generate_partial_witness(inputs, prover_data, common_data)
    );

    let public_inputs = partition_witness.get_targets(&prover_data.public_inputs);
    let public_inputs_hash = hash_n_to_hash(public_inputs.clone(), true);

    if cfg!(debug_assertions) {
        // Display the marked targets for debugging purposes.
        for m in &prover_data.marked_targets {
            m.display(&partition_witness);
        }
    }

    let witness = timed!(
        timing,
        "compute full witness",
        partition_witness.full_witness()
    );

    let wires_values: Vec<PolynomialValues<F>> = timed!(
        timing,
        "compute wire polynomials",
        witness
            .wire_values
            .par_iter()
            .map(|column| PolynomialValues::new(column.clone()))
            .collect()
    );

    let wires_commitment = timed!(
        timing,
        "compute wires commitment",
        PolynomialBatchCommitment::from_values(
            wires_values,
            config.rate_bits,
            config.zero_knowledge & PlonkPolynomials::WIRES.blinding,
            config.cap_height,
            timing,
            prover_data.fft_root_table.as_ref(),
        )
    );

    let mut challenger = Challenger::new();

    // Observe the instance.
    challenger.observe_hash(&common_data.circuit_digest);
    challenger.observe_hash(&public_inputs_hash);

    challenger.observe_cap(&wires_commitment.merkle_tree.cap);
    let betas = challenger.get_n_challenges(num_challenges);
    let gammas = challenger.get_n_challenges(num_challenges);

    assert!(
        common_data.quotient_degree_factor < common_data.config.num_routed_wires,
        "When the number of routed wires is smaller that the degree, we should change the logic to avoid computing partial products."
    );
    let mut partial_products_and_zs = timed!(
        timing,
        "compute partial products",
        all_wires_permutation_partial_products(&witness, &betas, &gammas, prover_data, common_data)
    );

    // Z is expected at the front of our batch; see `zs_range` and `partial_products_range`.
    let plonk_z_vecs = partial_products_and_zs
        .iter_mut()
        .map(|partial_products_and_z| partial_products_and_z.pop().unwrap())
        .collect();
    let zs_partial_products = [plonk_z_vecs, partial_products_and_zs.concat()].concat();

    let partial_products_and_zs_commitment = timed!(
        timing,
        "commit to partial products and Z's",
        PolynomialBatchCommitment::from_values(
            zs_partial_products,
            config.rate_bits,
            config.zero_knowledge & PlonkPolynomials::ZS_PARTIAL_PRODUCTS.blinding,
            config.cap_height,
            timing,
            prover_data.fft_root_table.as_ref(),
        )
    );

    challenger.observe_cap(&partial_products_and_zs_commitment.merkle_tree.cap);

    let alphas = challenger.get_n_challenges(num_challenges);

    let quotient_polys = timed!(
        timing,
        "compute quotient polys",
        compute_quotient_polys(
            common_data,
            prover_data,
            &public_inputs_hash,
            &wires_commitment,
            &partial_products_and_zs_commitment,
            &betas,
            &gammas,
            &alphas,
        )
    );

    // Compute the quotient polynomials, aka `t` in the Plonk paper.
    let all_quotient_poly_chunks = timed!(
        timing,
        "split up quotient polys",
        quotient_polys
            .into_par_iter()
            .flat_map(|mut quotient_poly| {
                quotient_poly.trim();
                quotient_poly.pad(quotient_degree).expect(
                    "Quotient has failed, the vanishing polynomial is not divisible by `Z_H",
                );
                // Split t into degree-n chunks.
                quotient_poly.chunks(degree)
            })
            .collect()
    );

    let quotient_polys_commitment = timed!(
        timing,
        "commit to quotient polys",
        PolynomialBatchCommitment::from_coeffs(
            all_quotient_poly_chunks,
            config.rate_bits,
            config.zero_knowledge & PlonkPolynomials::QUOTIENT.blinding,
            config.cap_height,
            timing,
            prover_data.fft_root_table.as_ref(),
        )
    );

    challenger.observe_cap(&quotient_polys_commitment.merkle_tree.cap);

    let zeta = challenger.get_extension_challenge();

    let (opening_proof, openings) = timed!(
        timing,
        "compute opening proofs",
        PolynomialBatchCommitment::open_plonk(
            &[
                &prover_data.constants_sigmas_commitment,
                &wires_commitment,
                &partial_products_and_zs_commitment,
                &quotient_polys_commitment,
            ],
            zeta,
            &mut challenger,
            common_data,
            timing,
        )
    );

    let proof = Proof {
        wires_cap: wires_commitment.merkle_tree.cap,
        plonk_zs_partial_products_cap: partial_products_and_zs_commitment.merkle_tree.cap,
        quotient_polys_cap: quotient_polys_commitment.merkle_tree.cap,
        openings,
        opening_proof,
    };
    Ok(ProofWithPublicInputs {
        proof,
        public_inputs,
    })
}

/// Compute the partial products used in the `Z` polynomials.
fn all_wires_permutation_partial_products<F: RichField + Extendable<D>, const D: usize>(
    witness: &MatrixWitness<F>,
    betas: &[F],
    gammas: &[F],
    prover_data: &ProverOnlyCircuitData<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Vec<Vec<PolynomialValues<F>>> {
    (0..common_data.config.num_challenges)
        .map(|i| {
            wires_permutation_partial_products_and_zs(
                witness,
                betas[i],
                gammas[i],
                prover_data,
                common_data,
            )
        })
        .collect()
}

/// Compute the partial products used in the `Z` polynomial.
/// Returns the polynomials interpolating `partial_products(f / g)`
/// where `f, g` are the products in the definition of `Z`: `Z(g^i) = f / g`.
fn wires_permutation_partial_products_and_zs<F: RichField + Extendable<D>, const D: usize>(
    witness: &MatrixWitness<F>,
    beta: F,
    gamma: F,
    prover_data: &ProverOnlyCircuitData<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Vec<PolynomialValues<F>> {
    let degree = common_data.quotient_degree_factor;
    let subgroup = &prover_data.subgroup;
    let k_is = &common_data.k_is;
    let (num_prods, _final_num_prod) = common_data.num_partial_products;
    let all_quotient_chunk_products = subgroup
        .par_iter()
        .enumerate()
        .map(|(i, &x)| {
            let s_sigmas = &prover_data.sigmas[i];
            let numerators = (0..common_data.config.num_routed_wires).map(|j| {
                let wire_value = witness.get_wire(i, j);
                let k_i = k_is[j];
                let s_id = k_i * x;
                wire_value + beta * s_id + gamma
            });
            let denominators = (0..common_data.config.num_routed_wires)
                .map(|j| {
                    let wire_value = witness.get_wire(i, j);
                    let s_sigma = s_sigmas[j];
                    wire_value + beta * s_sigma + gamma
                })
                .collect::<Vec<_>>();
            let denominator_invs = F::batch_multiplicative_inverse(&denominators);
            let quotient_values = numerators
                .zip(denominator_invs)
                .map(|(num, den_inv)| num * den_inv)
                .collect::<Vec<_>>();

            quotient_chunk_products(&quotient_values, degree)
        })
        .collect::<Vec<_>>();

    let mut z_x = F::ONE;
    let mut all_partial_products_and_zs = Vec::new();
    for quotient_chunk_products in all_quotient_chunk_products {
        let mut partial_products_and_z_gx =
            partial_products_and_z_gx(z_x, &quotient_chunk_products);
        // The last term is Z(gx), but we replace it with Z(x), otherwise Z would end up shifted.
        swap(&mut z_x, &mut partial_products_and_z_gx[num_prods]);
        all_partial_products_and_zs.push(partial_products_and_z_gx);
    }

    transpose(&all_partial_products_and_zs)
        .into_par_iter()
        .map(PolynomialValues::new)
        .collect()
}

const BATCH_SIZE: usize = 32;

fn compute_quotient_polys<'a, F: RichField + Extendable<D>, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    prover_data: &'a ProverOnlyCircuitData<F, D>,
    public_inputs_hash: &HashOut<F>,
    wires_commitment: &'a PolynomialBatchCommitment<F>,
    zs_partial_products_commitment: &'a PolynomialBatchCommitment<F>,
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<PolynomialCoeffs<F>> {
    let num_challenges = common_data.config.num_challenges;
    let max_degree_bits = log2_ceil(common_data.quotient_degree_factor);
    assert!(
        max_degree_bits <= common_data.config.rate_bits,
        "Having constraints of degree higher than the rate is not supported yet. \
        If we need this in the future, we can precompute the larger LDE before computing the `ListPolynomialCommitment`s."
    );

    // We reuse the LDE computed in `ListPolynomialCommitment` and extract every `step` points to get
    // an LDE matching `max_filtered_constraint_degree`.
    let step = 1 << (common_data.config.rate_bits - max_degree_bits);
    // When opening the `Z`s polys at the "next" point in Plonk, need to look at the point `next_step`
    // steps away since we work on an LDE of degree `max_filtered_constraint_degree`.
    let next_step = 1 << max_degree_bits;

    let points = F::two_adic_subgroup(common_data.degree_bits + max_degree_bits);
    let lde_size = points.len();

    // Retrieve the LDE values at index `i`.
    let get_at_index = |comm: &'a PolynomialBatchCommitment<F>, i: usize| -> &'a [F] {
        comm.get_lde_values(i * step)
    };

    let z_h_on_coset = ZeroPolyOnCoset::new(common_data.degree_bits, max_degree_bits);

    let points_batches = points.par_chunks(BATCH_SIZE);
    let quotient_values: Vec<Vec<F>> = points_batches
        .enumerate()
        .map(|(batch_i, xs_batch)| {
            assert_eq!(xs_batch.len(), BATCH_SIZE);
            let indices_batch: Vec<usize> =
                (BATCH_SIZE * batch_i..BATCH_SIZE * (batch_i + 1)).collect();

            let mut shifted_xs_batch = Vec::with_capacity(xs_batch.len());
            let mut local_zs_batch = Vec::with_capacity(xs_batch.len());
            let mut next_zs_batch = Vec::with_capacity(xs_batch.len());
            let mut partial_products_batch = Vec::with_capacity(xs_batch.len());
            let mut s_sigmas_batch = Vec::with_capacity(xs_batch.len());

            let mut local_constants_batch_refs = Vec::with_capacity(xs_batch.len());
            let mut local_wires_batch_refs = Vec::with_capacity(xs_batch.len());

            for (&i, &x) in indices_batch.iter().zip(xs_batch) {
                let shifted_x = F::coset_shift() * x;
                let i_next = (i + next_step) % lde_size;
                let local_constants_sigmas =
                    get_at_index(&prover_data.constants_sigmas_commitment, i);
                let local_constants = &local_constants_sigmas[common_data.constants_range()];
                let s_sigmas = &local_constants_sigmas[common_data.sigmas_range()];
                let local_wires = get_at_index(wires_commitment, i);
                let local_zs_partial_products = get_at_index(zs_partial_products_commitment, i);
                let local_zs = &local_zs_partial_products[common_data.zs_range()];
                let next_zs =
                    &get_at_index(zs_partial_products_commitment, i_next)[common_data.zs_range()];
                let partial_products =
                    &local_zs_partial_products[common_data.partial_products_range()];

                debug_assert_eq!(local_wires.len(), common_data.config.num_wires);
                debug_assert_eq!(local_zs.len(), num_challenges);

                local_constants_batch_refs.push(local_constants);
                local_wires_batch_refs.push(local_wires);

                shifted_xs_batch.push(shifted_x);
                local_zs_batch.push(local_zs);
                next_zs_batch.push(next_zs);
                partial_products_batch.push(partial_products);
                s_sigmas_batch.push(s_sigmas);
            }

            // NB (JN): I'm not sure how (in)efficient the below is. It needs measuring.
            let mut local_constants_batch =
                vec![F::ZERO; xs_batch.len() * local_constants_batch_refs[0].len()];
            for (i, constants) in local_constants_batch_refs.iter().enumerate() {
                for (j, &constant) in constants.iter().enumerate() {
                    local_constants_batch[i + j * xs_batch.len()] = constant;
                }
            }

            let mut local_wires_batch =
                vec![F::ZERO; xs_batch.len() * local_wires_batch_refs[0].len()];
            for (i, wires) in local_wires_batch_refs.iter().enumerate() {
                for (j, &wire) in wires.iter().enumerate() {
                    local_wires_batch[i + j * xs_batch.len()] = wire;
                }
            }

            let vars_batch = EvaluationVarsBaseBatch::new(
                xs_batch.len(),
                &local_constants_batch,
                &local_wires_batch,
                public_inputs_hash,
            );

            let mut quotient_values_batch = eval_vanishing_poly_base_batch(
                common_data,
                &indices_batch,
                &shifted_xs_batch,
                vars_batch,
                &local_zs_batch,
                &next_zs_batch,
                &partial_products_batch,
                &s_sigmas_batch,
                betas,
                gammas,
                alphas,
                &z_h_on_coset,
            );

            for (&i, quotient_values) in indices_batch.iter().zip(quotient_values_batch.iter_mut())
            {
                let denominator_inv = z_h_on_coset.eval_inverse(i);
                quotient_values
                    .iter_mut()
                    .for_each(|v| *v *= denominator_inv);
            }
            quotient_values_batch
        })
        .flatten()
        .collect();

    transpose(&quotient_values)
        .into_par_iter()
        .map(PolynomialValues::new)
        .map(|values| values.coset_ifft(F::coset_shift()))
        .collect()
}
