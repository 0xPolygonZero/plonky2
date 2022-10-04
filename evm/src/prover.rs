use std::any::type_name;

use anyhow::{ensure, Result};
use maybe_rayon::*;
use plonky2::field::extension::Extendable;
use plonky2::field::packable::Packable;
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2::field::types::Field;
use plonky2::field::zero_poly_coset::ZeroPolyOnCoset;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::{GenericConfig, Hasher};
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;
use plonky2_util::{log2_ceil, log2_strict};

use crate::all_stark::{AllStark, Table, NUM_TABLES};
use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::cpu::cpu_stark::CpuStark;
use crate::cross_table_lookup::{cross_table_lookup_data, CtlCheckVars, CtlData};
use crate::generation::{generate_traces, GenerationInputs};
use crate::keccak::keccak_stark::KeccakStark;
use crate::keccak_memory::keccak_memory_stark::KeccakMemoryStark;
use crate::logic::LogicStark;
use crate::memory::memory_stark::MemoryStark;
use crate::permutation::PermutationCheckVars;
use crate::permutation::{
    compute_permutation_z_polys, get_n_grand_product_challenge_sets, GrandProductChallengeSet,
};
use crate::proof::{AllProof, PublicValues, StarkOpeningSet, StarkProof};
use crate::stark::Stark;
use crate::vanishing_poly::eval_vanishing_poly;
use crate::vars::StarkEvaluationVars;

/// Generate traces, then create all STARK proofs.
pub fn prove<F, C, const D: usize>(
    all_stark: &AllStark<F, D>,
    config: &StarkConfig,
    inputs: GenerationInputs,
    timing: &mut TimingTree,
) -> Result<AllProof<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    [(); C::Hasher::HASH_SIZE]:,
    [(); CpuStark::<F, D>::COLUMNS]:,
    [(); KeccakStark::<F, D>::COLUMNS]:,
    [(); KeccakMemoryStark::<F, D>::COLUMNS]:,
    [(); LogicStark::<F, D>::COLUMNS]:,
    [(); MemoryStark::<F, D>::COLUMNS]:,
{
    let (traces, public_values) = generate_traces(all_stark, inputs, config, timing);
    prove_with_traces(all_stark, config, traces, public_values, timing)
}

/// Compute all STARK proofs.
pub(crate) fn prove_with_traces<F, C, const D: usize>(
    all_stark: &AllStark<F, D>,
    config: &StarkConfig,
    trace_poly_values: [Vec<PolynomialValues<F>>; NUM_TABLES],
    public_values: PublicValues,
    timing: &mut TimingTree,
) -> Result<AllProof<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    [(); C::Hasher::HASH_SIZE]:,
    [(); CpuStark::<F, D>::COLUMNS]:,
    [(); KeccakStark::<F, D>::COLUMNS]:,
    [(); KeccakMemoryStark::<F, D>::COLUMNS]:,
    [(); LogicStark::<F, D>::COLUMNS]:,
    [(); MemoryStark::<F, D>::COLUMNS]:,
{
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;

    let trace_commitments = timed!(
        timing,
        "compute trace commitments",
        trace_poly_values
            .iter()
            .map(|trace| {
                PolynomialBatch::<F, C, D>::from_values(
                    // TODO: Cloning this isn't great; consider having `from_values` accept a reference,
                    // or having `compute_permutation_z_polys` read trace values from the `PolynomialBatch`.
                    trace.clone(),
                    rate_bits,
                    false,
                    cap_height,
                    timing,
                    None,
                )
            })
            .collect::<Vec<_>>()
    );

    let trace_caps = trace_commitments
        .iter()
        .map(|c| c.merkle_tree.cap.clone())
        .collect::<Vec<_>>();
    let mut challenger = Challenger::<F, C::Hasher>::new();
    for cap in &trace_caps {
        challenger.observe_cap(cap);
    }

    let ctl_data_per_table = cross_table_lookup_data::<F, C, D>(
        config,
        &trace_poly_values,
        &all_stark.cross_table_lookups,
        &mut challenger,
    );

    let cpu_proof = prove_single_table(
        &all_stark.cpu_stark,
        config,
        &trace_poly_values[Table::Cpu as usize],
        &trace_commitments[Table::Cpu as usize],
        &ctl_data_per_table[Table::Cpu as usize],
        &mut challenger,
        timing,
    )?;
    let keccak_proof = prove_single_table(
        &all_stark.keccak_stark,
        config,
        &trace_poly_values[Table::Keccak as usize],
        &trace_commitments[Table::Keccak as usize],
        &ctl_data_per_table[Table::Keccak as usize],
        &mut challenger,
        timing,
    )?;
    let keccak_memory_proof = prove_single_table(
        &all_stark.keccak_memory_stark,
        config,
        &trace_poly_values[Table::KeccakMemory as usize],
        &trace_commitments[Table::KeccakMemory as usize],
        &ctl_data_per_table[Table::KeccakMemory as usize],
        &mut challenger,
        timing,
    )?;
    let logic_proof = prove_single_table(
        &all_stark.logic_stark,
        config,
        &trace_poly_values[Table::Logic as usize],
        &trace_commitments[Table::Logic as usize],
        &ctl_data_per_table[Table::Logic as usize],
        &mut challenger,
        timing,
    )?;
    let memory_proof = prove_single_table(
        &all_stark.memory_stark,
        config,
        &trace_poly_values[Table::Memory as usize],
        &trace_commitments[Table::Memory as usize],
        &ctl_data_per_table[Table::Memory as usize],
        &mut challenger,
        timing,
    )?;

    let stark_proofs = [
        cpu_proof,
        keccak_proof,
        keccak_memory_proof,
        logic_proof,
        memory_proof,
    ];

    Ok(AllProof {
        stark_proofs,
        public_values,
    })
}

/// Compute proof for a single STARK table.
pub(crate) fn prove_single_table<F, C, S, const D: usize>(
    stark: &S,
    config: &StarkConfig,
    trace_poly_values: &[PolynomialValues<F>],
    trace_commitment: &PolynomialBatch<F, C, D>,
    ctl_data: &CtlData<F>,
    challenger: &mut Challenger<F, C::Hasher>,
    timing: &mut TimingTree,
) -> Result<StarkProof<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); C::Hasher::HASH_SIZE]:,
    [(); S::COLUMNS]:,
{
    let degree = trace_poly_values[0].len();
    let degree_bits = log2_strict(degree);
    let fri_params = config.fri_params(degree_bits);
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    assert!(
        fri_params.total_arities() <= degree_bits + rate_bits - cap_height,
        "FRI total reduction arity is too large.",
    );

    // Permutation arguments.
    let permutation_challenges = stark.uses_permutation_args().then(|| {
        get_n_grand_product_challenge_sets(
            challenger,
            config.num_challenges,
            stark.permutation_batch_size(),
        )
    });
    let permutation_zs = permutation_challenges.as_ref().map(|challenges| {
        timed!(
            timing,
            "compute permutation Z(x) polys",
            compute_permutation_z_polys::<F, C, S, D>(stark, config, trace_poly_values, challenges)
        )
    });
    let num_permutation_zs = permutation_zs.as_ref().map(|v| v.len()).unwrap_or(0);

    let z_polys = match permutation_zs {
        None => ctl_data.z_polys(),
        Some(mut permutation_zs) => {
            permutation_zs.extend(ctl_data.z_polys());
            permutation_zs
        }
    };
    assert!(!z_polys.is_empty(), "No CTL?");

    let permutation_ctl_zs_commitment = timed!(
        timing,
        "compute Zs commitment",
        PolynomialBatch::from_values(
            z_polys,
            rate_bits,
            false,
            config.fri_config.cap_height,
            timing,
            None,
        )
    );

    let permutation_ctl_zs_cap = permutation_ctl_zs_commitment.merkle_tree.cap.clone();
    challenger.observe_cap(&permutation_ctl_zs_cap);

    let alphas = challenger.get_n_challenges(config.num_challenges);
    if cfg!(test) {
        check_constraints(
            stark,
            trace_commitment,
            &permutation_ctl_zs_commitment,
            permutation_challenges.as_ref(),
            ctl_data,
            alphas.clone(),
            degree_bits,
            num_permutation_zs,
            config,
        );
    }
    let quotient_polys = timed!(
        timing,
        "compute quotient polys",
        compute_quotient_polys::<F, <F as Packable>::Packing, C, S, D>(
            stark,
            trace_commitment,
            &permutation_ctl_zs_commitment,
            permutation_challenges.as_ref(),
            ctl_data,
            alphas,
            degree_bits,
            num_permutation_zs,
            config,
        )
    );
    let all_quotient_chunks = timed!(
        timing,
        "split quotient polys",
        quotient_polys
            .into_par_iter()
            .flat_map(|mut quotient_poly| {
                quotient_poly
                    .trim_to_len(degree * stark.quotient_degree_factor())
                    .expect(
                        "Quotient has failed, the vanishing polynomial is not divisible by Z_H",
                    );
                // Split quotient into degree-n chunks.
                quotient_poly.chunks(degree)
            })
            .collect()
    );
    let quotient_commitment = timed!(
        timing,
        "compute quotient commitment",
        PolynomialBatch::from_coeffs(
            all_quotient_chunks,
            rate_bits,
            false,
            config.fri_config.cap_height,
            timing,
            None,
        )
    );
    let quotient_polys_cap = quotient_commitment.merkle_tree.cap.clone();
    challenger.observe_cap(&quotient_polys_cap);

    let zeta = challenger.get_extension_challenge::<D>();
    // To avoid leaking witness data, we want to ensure that our opening locations, `zeta` and
    // `g * zeta`, are not in our subgroup `H`. It suffices to check `zeta` only, since
    // `(g * zeta)^n = zeta^n`, where `n` is the order of `g`.
    let g = F::primitive_root_of_unity(degree_bits);
    ensure!(
        zeta.exp_power_of_2(degree_bits) != F::Extension::ONE,
        "Opening point is in the subgroup."
    );

    let openings = StarkOpeningSet::new(
        zeta,
        g,
        trace_commitment,
        &permutation_ctl_zs_commitment,
        &quotient_commitment,
        degree_bits,
        stark.num_permutation_batches(config),
    );
    challenger.observe_openings(&openings.to_fri_openings());

    let initial_merkle_trees = vec![
        trace_commitment,
        &permutation_ctl_zs_commitment,
        &quotient_commitment,
    ];

    let opening_proof = timed!(
        timing,
        "compute openings proof",
        PolynomialBatch::prove_openings(
            &stark.fri_instance(zeta, g, degree_bits, ctl_data.len(), config),
            &initial_merkle_trees,
            challenger,
            &fri_params,
            timing,
        )
    );

    Ok(StarkProof {
        trace_cap: trace_commitment.merkle_tree.cap.clone(),
        permutation_ctl_zs_cap,
        quotient_polys_cap,
        openings,
        opening_proof,
    })
}

/// Computes the quotient polynomials `(sum alpha^i C_i(x)) / Z_H(x)` for `alpha` in `alphas`,
/// where the `C_i`s are the Stark constraints.
fn compute_quotient_polys<'a, F, P, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &'a PolynomialBatch<F, C, D>,
    permutation_ctl_zs_commitment: &'a PolynomialBatch<F, C, D>,
    permutation_challenges: Option<&'a Vec<GrandProductChallengeSet<F>>>,
    ctl_data: &CtlData<F>,
    alphas: Vec<F>,
    degree_bits: usize,
    num_permutation_zs: usize,
    config: &StarkConfig,
) -> Vec<PolynomialCoeffs<F>>
where
    F: RichField + Extendable<D>,
    P: PackedField<Scalar = F>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
{
    let degree = 1 << degree_bits;
    let rate_bits = config.fri_config.rate_bits;

    let quotient_degree_bits = log2_ceil(stark.quotient_degree_factor());
    assert!(
        quotient_degree_bits <= rate_bits,
        "Having constraints of degree higher than the rate is not supported yet."
    );
    let step = 1 << (rate_bits - quotient_degree_bits);
    // When opening the `Z`s polys at the "next" point, need to look at the point `next_step` steps away.
    let next_step = 1 << quotient_degree_bits;

    // Evaluation of the first Lagrange polynomial on the LDE domain.
    let lagrange_first = PolynomialValues::selector(degree, 0).lde_onto_coset(quotient_degree_bits);
    // Evaluation of the last Lagrange polynomial on the LDE domain.
    let lagrange_last =
        PolynomialValues::selector(degree, degree - 1).lde_onto_coset(quotient_degree_bits);

    let z_h_on_coset = ZeroPolyOnCoset::<F>::new(degree_bits, quotient_degree_bits);

    // Retrieve the LDE values at index `i`.
    let get_trace_values_packed = |i_start| -> [P; S::COLUMNS] {
        trace_commitment
            .get_lde_values_packed(i_start, step)
            .try_into()
            .unwrap()
    };

    // Last element of the subgroup.
    let last = F::primitive_root_of_unity(degree_bits).inverse();
    let size = degree << quotient_degree_bits;
    let coset = F::cyclic_subgroup_coset_known_order(
        F::primitive_root_of_unity(degree_bits + quotient_degree_bits),
        F::coset_shift(),
        size,
    );

    // We will step by `P::WIDTH`, and in each iteration, evaluate the quotient polynomial at
    // a batch of `P::WIDTH` points.
    let quotient_values = (0..size)
        .into_par_iter()
        .step_by(P::WIDTH)
        .flat_map_iter(|i_start| {
            let i_next_start = (i_start + next_step) % size;
            let i_range = i_start..i_start + P::WIDTH;

            let x = *P::from_slice(&coset[i_range.clone()]);
            let z_last = x - last;
            let lagrange_basis_first = *P::from_slice(&lagrange_first.values[i_range.clone()]);
            let lagrange_basis_last = *P::from_slice(&lagrange_last.values[i_range]);

            let mut consumer = ConstraintConsumer::new(
                alphas.clone(),
                z_last,
                lagrange_basis_first,
                lagrange_basis_last,
            );
            let vars = StarkEvaluationVars {
                local_values: &get_trace_values_packed(i_start),
                next_values: &get_trace_values_packed(i_next_start),
            };
            let permutation_check_vars =
                permutation_challenges.map(|permutation_challenge_sets| PermutationCheckVars {
                    local_zs: permutation_ctl_zs_commitment.get_lde_values_packed(i_start, step)
                        [..num_permutation_zs]
                        .to_vec(),
                    next_zs: permutation_ctl_zs_commitment
                        .get_lde_values_packed(i_next_start, step)[..num_permutation_zs]
                        .to_vec(),
                    permutation_challenge_sets: permutation_challenge_sets.to_vec(),
                });
            let ctl_vars = ctl_data
                .zs_columns
                .iter()
                .enumerate()
                .map(|(i, zs_columns)| CtlCheckVars::<F, F, P, 1> {
                    local_z: permutation_ctl_zs_commitment.get_lde_values_packed(i_start, step)
                        [num_permutation_zs + i],
                    next_z: permutation_ctl_zs_commitment.get_lde_values_packed(i_next_start, step)
                        [num_permutation_zs + i],
                    challenges: zs_columns.challenge,
                    columns: &zs_columns.columns,
                    filter_column: &zs_columns.filter_column,
                })
                .collect::<Vec<_>>();
            eval_vanishing_poly::<F, F, P, C, S, D, 1>(
                stark,
                config,
                vars,
                permutation_check_vars,
                &ctl_vars,
                &mut consumer,
            );
            let mut constraints_evals = consumer.accumulators();
            // We divide the constraints evaluations by `Z_H(x)`.
            let denominator_inv: P = z_h_on_coset.eval_inverse_packed(i_start);
            for eval in &mut constraints_evals {
                *eval *= denominator_inv;
            }

            let num_challenges = alphas.len();

            (0..P::WIDTH).into_iter().map(move |i| {
                (0..num_challenges)
                    .map(|j| constraints_evals[j].as_slice()[i])
                    .collect()
            })
        })
        .collect::<Vec<_>>();

    transpose(&quotient_values)
        .into_par_iter()
        .map(PolynomialValues::new)
        .map(|values| values.coset_ifft(F::coset_shift()))
        .collect()
}

/// Check that all constraints evaluate to zero on `H`.
/// Can also be used to check the degree of the constraints by evaluating on a larger subgroup.
fn check_constraints<'a, F, C, S, const D: usize>(
    stark: &S,
    trace_commitment: &'a PolynomialBatch<F, C, D>,
    permutation_ctl_zs_commitment: &'a PolynomialBatch<F, C, D>,
    permutation_challenges: Option<&'a Vec<GrandProductChallengeSet<F>>>,
    ctl_data: &CtlData<F>,
    alphas: Vec<F>,
    degree_bits: usize,
    num_permutation_zs: usize,
    config: &StarkConfig,
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
{
    let degree = 1 << degree_bits;
    let rate_bits = 0; // Set this to higher value to check constraint degree.

    let size = degree << rate_bits;
    let step = 1 << rate_bits;

    // Evaluation of the first Lagrange polynomial.
    let lagrange_first = PolynomialValues::selector(degree, 0).lde(rate_bits);
    // Evaluation of the last Lagrange polynomial.
    let lagrange_last = PolynomialValues::selector(degree, degree - 1).lde(rate_bits);

    let subgroup = F::two_adic_subgroup(degree_bits + rate_bits);

    // Get the evaluations of a batch of polynomials over our subgroup.
    let get_subgroup_evals = |comm: &PolynomialBatch<F, C, D>| -> Vec<Vec<F>> {
        let values = comm
            .polynomials
            .par_iter()
            .map(|coeffs| coeffs.clone().fft().values)
            .collect::<Vec<_>>();
        transpose(&values)
    };

    let trace_subgroup_evals = get_subgroup_evals(trace_commitment);
    let permutation_ctl_zs_subgroup_evals = get_subgroup_evals(permutation_ctl_zs_commitment);

    // Last element of the subgroup.
    let last = F::primitive_root_of_unity(degree_bits).inverse();

    let constraint_values = (0..size)
        .map(|i| {
            let i_next = (i + step) % size;

            let x = subgroup[i];
            let z_last = x - last;
            let lagrange_basis_first = lagrange_first.values[i];
            let lagrange_basis_last = lagrange_last.values[i];

            let mut consumer = ConstraintConsumer::new(
                alphas.clone(),
                z_last,
                lagrange_basis_first,
                lagrange_basis_last,
            );
            let vars = StarkEvaluationVars {
                local_values: trace_subgroup_evals[i].as_slice().try_into().unwrap(),
                next_values: trace_subgroup_evals[i_next].as_slice().try_into().unwrap(),
            };
            let permutation_check_vars =
                permutation_challenges.map(|permutation_challenge_sets| PermutationCheckVars {
                    local_zs: permutation_ctl_zs_subgroup_evals[i][..num_permutation_zs].to_vec(),
                    next_zs: permutation_ctl_zs_subgroup_evals[i_next][..num_permutation_zs]
                        .to_vec(),
                    permutation_challenge_sets: permutation_challenge_sets.to_vec(),
                });

            let ctl_vars = ctl_data
                .zs_columns
                .iter()
                .enumerate()
                .map(|(iii, zs_columns)| CtlCheckVars::<F, F, F, 1> {
                    local_z: permutation_ctl_zs_subgroup_evals[i][num_permutation_zs + iii],
                    next_z: permutation_ctl_zs_subgroup_evals[i_next][num_permutation_zs + iii],
                    challenges: zs_columns.challenge,
                    columns: &zs_columns.columns,
                    filter_column: &zs_columns.filter_column,
                })
                .collect::<Vec<_>>();
            eval_vanishing_poly::<F, F, F, C, S, D, 1>(
                stark,
                config,
                vars,
                permutation_check_vars,
                &ctl_vars,
                &mut consumer,
            );
            consumer.accumulators()
        })
        .collect::<Vec<_>>();

    for v in constraint_values {
        assert!(
            v.iter().all(|x| x.is_zero()),
            "Constraint failed in {}",
            type_name::<S>()
        );
    }
}
