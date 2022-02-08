use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::util::reducing::ReducingFactorTarget;
use plonky2::with_context;

use crate::config::StarkConfig;
use crate::constraint_consumer::RecursiveConstraintConsumer;
use crate::proof::{
    StarkOpeningSetTarget, StarkProofChallengesTarget, StarkProofWithPublicInputsTarget,
};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub fn verify_stark_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: S,
    proof_with_pis: StarkProofWithPublicInputsTarget<D>,
    inner_config: &StarkConfig,
) where
    C::Hasher: AlgebraicHasher<F>,
    [(); { S::COLUMNS }]:,
    [(); { S::PUBLIC_INPUTS }]:,
{
    assert_eq!(proof_with_pis.public_inputs.len(), S::PUBLIC_INPUTS);
    let degree_bits = proof_with_pis.proof.recover_degree_bits(inner_config);
    let challenges = proof_with_pis.get_challenges::<F, C>(builder, inner_config, degree_bits);

    verify_stark_proof_with_challenges::<F, C, S, D>(
        builder,
        stark,
        proof_with_pis,
        challenges,
        inner_config,
        degree_bits,
    );
}

/// Recursively verifies an inner proof.
fn verify_stark_proof_with_challenges<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: S,
    proof_with_pis: StarkProofWithPublicInputsTarget<D>,
    challenges: StarkProofChallengesTarget<D>,
    inner_config: &StarkConfig,
    degree_bits: usize,
) where
    C::Hasher: AlgebraicHasher<F>,
    [(); { S::COLUMNS }]:,
    [(); { S::PUBLIC_INPUTS }]:,
{
    let one = builder.one_extension();

    let StarkProofWithPublicInputsTarget {
        proof,
        public_inputs,
    } = proof_with_pis;
    let local_values = &proof.openings.local_values;
    let next_values = &proof.openings.local_values;
    let StarkOpeningSetTarget {
        local_values,
        next_values,
        permutation_zs,
        permutation_zs_right,
        quotient_polys,
    } = &proof.openings;
    let vars = StarkEvaluationTargets {
        local_values: &local_values.to_vec().try_into().unwrap(),
        next_values: &next_values.to_vec().try_into().unwrap(),
        public_inputs: &public_inputs
            .into_iter()
            .map(|t| builder.convert_to_ext(t))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    };
    let (l_1, l_last) =
        eval_l_1_and_l_last_recursively(builder, degree_bits, challenges.stark_zeta);
    let last =
        builder.constant_extension(F::Extension::primitive_root_of_unity(degree_bits).inverse());
    let z_last = builder.sub_extension(challenges.stark_zeta, last);
    let mut consumer = RecursiveConstraintConsumer::<F, D>::new(
        builder.zero_extension(),
        challenges.stark_alphas,
        z_last,
        l_1,
        l_last,
    );
    stark.eval_ext_recursively(builder, vars, &mut consumer);
    let vanishing_polys_zeta = consumer.accumulators();

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let quotient_polys_zeta = &proof.openings.quotient_polys;
    let zeta_pow_deg = builder.exp_power_of_2_extension(challenges.stark_zeta, degree_bits);
    let mut scale = ReducingFactorTarget::new(zeta_pow_deg);
    let z_h_zeta = builder.sub_extension(zeta_pow_deg, one);
    for (i, chunk) in quotient_polys_zeta
        .chunks(stark.quotient_degree_factor())
        .enumerate()
    {
        let recombined_quotient = scale.reduce(chunk, builder);
        let computed_vanishing_poly = builder.mul_extension(z_h_zeta, recombined_quotient);
        builder.connect_extension(vanishing_polys_zeta[i], computed_vanishing_poly);
    }

    // TODO: Permutation polynomials.
    let merkle_caps = &[proof.trace_cap, proof.quotient_polys_cap];

    let fri_instance = stark.fri_instance_target(
        builder,
        challenges.stark_zeta,
        F::primitive_root_of_unity(degree_bits),
        inner_config.num_challenges,
    );
    builder.verify_fri_proof::<C>(
        &fri_instance,
        &proof.openings.to_fri_openings(),
        &challenges.fri_challenges,
        merkle_caps,
        &proof.opening_proof,
        &inner_config.fri_params(degree_bits),
    );
}

fn eval_l_1_and_l_last_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    log_n: usize,
    x: ExtensionTarget<D>,
) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
    let n = builder.constant_extension(F::Extension::from_canonical_usize(1 << log_n));
    let g = builder.constant_extension(F::Extension::primitive_root_of_unity(log_n));
    let x_pow_n = builder.exp_power_of_2_extension(x, log_n);
    let one = builder.one_extension();
    let z_x = builder.sub_extension(x_pow_n, one);
    let l_1_deno = builder.mul_sub_extension(n, x, n);
    let l_last_deno = builder.mul_sub_extension(g, x, one);
    let l_last_deno = builder.mul_extension(n, l_last_deno);

    (
        builder.div_extension(z_x, l_1_deno),
        builder.div_extension(z_x, l_last_deno),
    )
}

// pub fn add_virtual_proof_with_pis<InnerC: GenericConfig<D, F = F>>(
//     &mut self,
//     common_data: &CommonCircuitData<F, InnerC, D>,
// ) -> ProofWithPublicInputsTarget<D> {
//     let proof = self.add_virtual_proof(common_data);
//     let public_inputs = self.add_virtual_targets(common_data.num_public_inputs);
//     ProofWithPublicInputsTarget {
//         proof,
//         public_inputs,
//     }
// }
//
// fn add_virtual_proof<InnerC: GenericConfig<D, F = F>>(
//     &mut self,
//     common_data: &CommonCircuitData<F, InnerC, D>,
// ) -> ProofTarget<D> {
//     let config = &common_data.config;
//     let fri_params = &common_data.fri_params;
//     let cap_height = fri_params.config.cap_height;
//
//     let num_leaves_per_oracle = &[
//         common_data.num_preprocessed_polys(),
//         config.num_wires,
//         common_data.num_zs_partial_products_polys(),
//         common_data.num_quotient_polys(),
//     ];
//
//     ProofTarget {
//         wires_cap: self.add_virtual_cap(cap_height),
//         plonk_zs_partial_products_cap: self.add_virtual_cap(cap_height),
//         quotient_polys_cap: self.add_virtual_cap(cap_height),
//         openings: self.add_opening_set(common_data),
//         opening_proof: self.add_virtual_fri_proof(num_leaves_per_oracle, fri_params),
//     }
// }
//
// fn add_opening_set<InnerC: GenericConfig<D, F = F>>(
//     &mut self,
//     common_data: &CommonCircuitData<F, InnerC, D>,
// ) -> OpeningSetTarget<D> {
//     let config = &common_data.config;
//     let num_challenges = config.num_challenges;
//     let total_partial_products = num_challenges * common_data.num_partial_products;
//     OpeningSetTarget {
//         constants: self.add_virtual_extension_targets(common_data.num_constants),
//         plonk_sigmas: self.add_virtual_extension_targets(config.num_routed_wires),
//         wires: self.add_virtual_extension_targets(config.num_wires),
//         plonk_zs: self.add_virtual_extension_targets(num_challenges),
//         plonk_zs_right: self.add_virtual_extension_targets(num_challenges),
//         partial_products: self.add_virtual_extension_targets(total_partial_products),
//         quotient_polys: self.add_virtual_extension_targets(common_data.num_quotient_polys()),
//     }
// }
