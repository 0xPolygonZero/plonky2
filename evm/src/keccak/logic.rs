use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::PrimeField64;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

pub(crate) fn xor<F: PrimeField64, const N: usize>(xs: [F; N]) -> F {
    xs.into_iter().fold(F::ZERO, |acc, x| {
        debug_assert!(x.is_zero() || x.is_one());
        F::from_canonical_u64(acc.to_canonical_u64() ^ x.to_canonical_u64())
    })
}

/// Computes the arithmetic generalization of `xor(x, y)`, i.e. `x + y - 2 x y`.
pub(crate) fn xor_gen<P: PackedField>(x: P, y: P) -> P {
    x + y - x * y.doubles()
}

/// Computes the arithmetic generalization of `xor3(x, y, z)`.
pub(crate) fn xor3_gen<P: PackedField>(x: P, y: P, z: P) -> P {
    xor_gen(x, xor_gen(y, z))
}

/// Computes the arithmetic generalization of `xor(x, y)`, i.e. `x + y - 2 x y`.
pub(crate) fn xor_gen_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    x: ExtensionTarget<D>,
    y: ExtensionTarget<D>,
) -> ExtensionTarget<D> {
    let sum = builder.add_extension(x, y);
    builder.arithmetic_extension(-F::TWO, F::ONE, x, y, sum)
}

/// Computes the arithmetic generalization of `xor(x, y)`, i.e. `x + y - 2 x y`.
pub(crate) fn xor3_gen_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    x: ExtensionTarget<D>,
    y: ExtensionTarget<D>,
    z: ExtensionTarget<D>,
) -> ExtensionTarget<D> {
    let x_xor_y = xor_gen_circuit(builder, x, y);
    xor_gen_circuit(builder, x_xor_y, z)
}

pub(crate) fn andn<F: PrimeField64>(x: F, y: F) -> F {
    debug_assert!(x.is_zero() || x.is_one());
    debug_assert!(y.is_zero() || y.is_one());
    let x = x.to_canonical_u64();
    let y = y.to_canonical_u64();
    F::from_canonical_u64(!x & y)
}

pub(crate) fn andn_gen<P: PackedField>(x: P, y: P) -> P {
    (P::ONES - x) * y
}

pub(crate) fn andn_gen_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    x: ExtensionTarget<D>,
    y: ExtensionTarget<D>,
) -> ExtensionTarget<D> {
    // (1 - x) y = -xy + y
    builder.arithmetic_extension(F::NEG_ONE, F::ONE, x, y, y)
}
