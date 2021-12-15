use std::marker::PhantomData;
use std::ops::Range;

use crate::field::extension_field::algebra::ExtensionAlgebra;
use crate::field::extension_field::target::{ExtensionAlgebraTarget, ExtensionTarget};
use crate::field::extension_field::Extendable;
use crate::field::extension_field::FieldExtension;
use crate::field::field_types::{Field, RichField};
use crate::gates::gate::Gate;
use crate::hash::poseidon::Poseidon;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

#[derive(Debug)]
pub struct PoseidonMdsGate<
    F: RichField + Extendable<D> + Poseidon<WIDTH>,
    const D: usize,
    const WIDTH: usize,
> where
    [(); WIDTH - 1]:,
{
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D> + Poseidon<WIDTH>, const D: usize, const WIDTH: usize>
    PoseidonMdsGate<F, D, WIDTH>
where
    [(); WIDTH - 1]:,
{
    pub fn new() -> Self {
        PoseidonMdsGate {
            _phantom: PhantomData,
        }
    }

    pub fn wires_input(i: usize) -> Range<usize> {
        assert!(i < WIDTH);
        i * D..(i + 1) * D
    }

    pub fn wires_output(i: usize) -> Range<usize> {
        assert!(i < WIDTH);
        (WIDTH + i) * D..(WIDTH + i + 1) * D
    }

    // Following are methods analogous to ones in `Poseidon`, but for extension algebras.

    /// Same as `mds_row_shf` for an extension algebra of `F`.
    fn mds_row_shf_algebra(
        r: usize,
        v: &[ExtensionAlgebra<F::Extension, D>; WIDTH],
    ) -> ExtensionAlgebra<F::Extension, D> {
        debug_assert!(r < WIDTH);
        let mut res = ExtensionAlgebra::ZERO;

        for i in 0..WIDTH {
            let coeff =
                F::Extension::from_canonical_u64(1 << <F as Poseidon<WIDTH>>::MDS_MATRIX_EXPS[i]);
            res += v[(i + r) % WIDTH].scalar_mul(coeff);
        }

        res
    }

    /// Same as `mds_row_shf_recursive` for an extension algebra of `F`.
    fn mds_row_shf_algebra_recursive(
        builder: &mut CircuitBuilder<F, D>,
        r: usize,
        v: &[ExtensionAlgebraTarget<D>; WIDTH],
    ) -> ExtensionAlgebraTarget<D> {
        debug_assert!(r < WIDTH);
        let mut res = builder.zero_ext_algebra();

        for i in 0..WIDTH {
            let coeff = builder.constant_extension(F::Extension::from_canonical_u64(
                1 << <F as Poseidon<WIDTH>>::MDS_MATRIX_EXPS[i],
            ));
            res = builder.scalar_mul_add_ext_algebra(coeff, v[(i + r) % WIDTH], res);
        }

        res
    }

    /// Same as `mds_layer` for an extension algebra of `F`.
    fn mds_layer_algebra(
        state: &[ExtensionAlgebra<F::Extension, D>; WIDTH],
    ) -> [ExtensionAlgebra<F::Extension, D>; WIDTH] {
        let mut result = [ExtensionAlgebra::ZERO; WIDTH];

        for r in 0..WIDTH {
            result[r] = Self::mds_row_shf_algebra(r, state);
        }

        result
    }

    /// Same as `mds_layer_recursive` for an extension algebra of `F`.
    fn mds_layer_algebra_recursive(
        builder: &mut CircuitBuilder<F, D>,
        state: &[ExtensionAlgebraTarget<D>; WIDTH],
    ) -> [ExtensionAlgebraTarget<D>; WIDTH] {
        let mut result = [builder.zero_ext_algebra(); WIDTH];

        for r in 0..WIDTH {
            result[r] = Self::mds_row_shf_algebra_recursive(builder, r, state);
        }

        result
    }
}

impl<F: RichField + Extendable<D> + Poseidon<WIDTH>, const D: usize, const WIDTH: usize> Gate<F, D>
    for PoseidonMdsGate<F, D, WIDTH>
where
    [(); WIDTH - 1]:,
{
    fn id(&self) -> String {
        format!("{:?}<WIDTH={}>", self, WIDTH)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let inputs: [_; WIDTH] = (0..WIDTH)
            .map(|i| vars.get_local_ext_algebra(Self::wires_input(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let computed_outputs = Self::mds_layer_algebra(&inputs);

        (0..WIDTH)
            .map(|i| vars.get_local_ext_algebra(Self::wires_output(i)))
            .zip(computed_outputs)
            .flat_map(|(out, computed_out)| (out - computed_out).to_basefield_array())
            .collect()
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let inputs: [_; WIDTH] = (0..WIDTH)
            .map(|i| vars.get_local_ext(Self::wires_input(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let computed_outputs = F::mds_layer_field(&inputs);

        (0..WIDTH)
            .map(|i| vars.get_local_ext(Self::wires_output(i)))
            .zip(computed_outputs)
            .flat_map(|(out, computed_out)| (out - computed_out).to_basefield_array())
            .collect()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let inputs: [_; WIDTH] = (0..WIDTH)
            .map(|i| vars.get_local_ext_algebra(Self::wires_input(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let computed_outputs = Self::mds_layer_algebra_recursive(builder, &inputs);

        (0..WIDTH)
            .map(|i| vars.get_local_ext_algebra(Self::wires_output(i)))
            .zip(computed_outputs)
            .flat_map(|(out, computed_out)| {
                builder
                    .sub_ext_algebra(out, computed_out)
                    .to_ext_target_array()
            })
            .collect()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = PoseidonMdsGenerator::<D, WIDTH> { gate_index };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        2 * D * WIDTH
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1
    }

    fn num_constraints(&self) -> usize {
        WIDTH * D
    }
}

#[derive(Clone, Debug)]
struct PoseidonMdsGenerator<const D: usize, const WIDTH: usize>
where
    [(); WIDTH - 1]:,
{
    gate_index: usize,
}

impl<F: RichField + Extendable<D> + Poseidon<WIDTH>, const D: usize, const WIDTH: usize>
    SimpleGenerator<F> for PoseidonMdsGenerator<D, WIDTH>
where
    [(); WIDTH - 1]:,
{
    fn dependencies(&self) -> Vec<Target> {
        (0..WIDTH)
            .flat_map(|i| {
                Target::wires_from_range(
                    self.gate_index,
                    PoseidonMdsGate::<F, D, WIDTH>::wires_input(i),
                )
            })
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let get_local_get_target =
            |wire_range| ExtensionTarget::from_range(self.gate_index, wire_range);
        let get_local_ext =
            |wire_range| witness.get_extension_target(get_local_get_target(wire_range));

        let inputs: [_; WIDTH] = (0..WIDTH)
            .map(|i| get_local_ext(PoseidonMdsGate::<F, D, WIDTH>::wires_input(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let outputs = F::mds_layer_field(&inputs);

        for (i, &out) in outputs.iter().enumerate() {
            out_buffer.set_extension_target(
                get_local_get_target(PoseidonMdsGate::<F, D, WIDTH>::wires_output(i)),
                out,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::poseidon_mds::PoseidonMdsGate;
    use crate::hash::hashing::SPONGE_WIDTH;

    #[test]
    fn low_degree() {
        type F = GoldilocksField;
        let gate = PoseidonMdsGate::<F, 4, SPONGE_WIDTH>::new();
        test_low_degree(gate)
    }

    #[test]
    fn eval_fns() -> anyhow::Result<()> {
        type F = GoldilocksField;
        let gate = PoseidonMdsGate::<F, 4, SPONGE_WIDTH>::new();
        test_eval_fns(gate)
    }
}
