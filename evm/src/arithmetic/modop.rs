use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::addmod;
use crate::arithmetic::columns;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ARITH_COLUMNS]) {
    let input_limbs = columns::ADDMOD_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let zero = [0u64; columns::N_LIMBS];
    let modulus_limbs = columns::ADDMOD_MODULUS.map(|c| lv[c].to_canonical_u64());

    addmod::generate_addmod(
        lv,
        input_limbs,
        zero,
        modulus_limbs,
        columns::MOD_OUTPUT,
        columns::MOD_QUO_INPUT,
        columns::MOD_AUX_INPUT,
    );
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_mod = lv[columns::IS_MOD];
    let input_limbs = columns::MOD_INPUT.map(|c| lv[c]);
    let modulus_limbs = columns::MOD_MODULUS.map(|c| lv[c]);
    let quot_limbs = columns::MOD_QUO_INPUT.map(|c| lv[c]);
    let aux_limbs = columns::MOD_AUX_INPUT.map(|c| lv[c]);
    let output_limbs = columns::MOD_OUTPUT.map(|c| lv[c]);

    // NB: This should be const, but Rust complains "can't use type
    // parameters from outer function;" which is non-sensical since
    // there is no outer function.
    let zero = [P::ZEROS; columns::N_LIMBS];

    addmod::eval_packed_generic_addmod(
        is_mod,
        input_limbs,
        zero,
        modulus_limbs,
        output_limbs,
        quot_limbs,
        aux_limbs,
        yield_constr,
    );
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mod = lv[columns::IS_MOD];
    let input_limbs = columns::MOD_INPUT.map(|c| lv[c]);
    let modulus_limbs = columns::MOD_MODULUS.map(|c| lv[c]);
    let quot_limbs = columns::MOD_QUO_INPUT.map(|c| lv[c]);
    let aux_limbs = columns::MOD_AUX_INPUT.map(|c| lv[c]);
    let output_limbs = columns::MOD_OUTPUT.map(|c| lv[c]);

    let zero_e = builder.zero_extension();
    let zero = [zero_e; columns::N_LIMBS];

    addmod::eval_ext_circuit_addmod(
        is_mod,
        input_limbs,
        zero,
        modulus_limbs,
        output_limbs,
        quot_limbs,
        aux_limbs,
        builder,
        yield_constr,
    );
}
