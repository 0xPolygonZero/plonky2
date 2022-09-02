use std::marker::PhantomData;
use std::ops::Range;

use plonky2_field::extension::algebra::PolynomialCoeffsAlgebra;
use plonky2_field::extension::{Extendable, FieldExtension};
use plonky2_field::interpolation::interpolant;
use plonky2_field::polynomial::PolynomialCoeffs;

use crate::gadgets::polynomial::PolynomialCoeffsExtAlgebraTarget;
use crate::gates::gate::Gate;
use crate::gates::interpolation::InterpolationGate;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// One of the instantiations of `InterpolationGate`: allows constraints of variable
/// degree, up to `1<<subgroup_bits`.
/// The higher degree is a tradeoff for less gates (`eval_unfiltered_recursively` for
/// this version uses less gates than `LowDegreeInterpolationGate`).
#[derive(Copy, Clone, Debug)]
pub struct HighDegreeInterpolationGate<F: RichField + Extendable<D>, const D: usize> {
    pub subgroup_bits: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> InterpolationGate<F, D>
    for HighDegreeInterpolationGate<F, D>
{
    fn new(subgroup_bits: usize) -> Self {
        Self {
            subgroup_bits,
            _phantom: PhantomData,
        }
    }

    fn num_points(&self) -> usize {
        1 << self.subgroup_bits
    }
}

impl<F: RichField + Extendable<D>, const D: usize> HighDegreeInterpolationGate<F, D> {
    /// End of wire indices, exclusive.
    fn end(&self) -> usize {
        self.start_coeffs() + self.num_points() * D
    }

    /// The domain of the points we're interpolating.
    fn coset(&self, shift: F) -> impl Iterator<Item = F> {
        let g = F::primitive_root_of_unity(self.subgroup_bits);
        let size = 1 << self.subgroup_bits;
        // Speed matters here, so we avoid `cyclic_subgroup_coset_known_order` which allocates.
        g.powers().take(size).map(move |x| x * shift)
    }

    /// The domain of the points we're interpolating.
    fn coset_ext(&self, shift: F::Extension) -> impl Iterator<Item = F::Extension> {
        let g = F::primitive_root_of_unity(self.subgroup_bits);
        let size = 1 << self.subgroup_bits;
        g.powers().take(size).map(move |x| shift.scalar_mul(x))
    }

    /// The domain of the points we're interpolating.
    fn coset_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        shift: ExtensionTarget<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let g = F::primitive_root_of_unity(self.subgroup_bits);
        let size = 1 << self.subgroup_bits;
        g.powers()
            .take(size)
            .map(move |x| {
                let subgroup_element = builder.constant(x);
                builder.scalar_mul_ext(subgroup_element, shift)
            })
            .collect()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D>
    for HighDegreeInterpolationGate<F, D>
{
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.wires_coeff(i)))
            .collect();
        let interpolant = PolynomialCoeffsAlgebra::new(coeffs);

        let coset = self.coset_ext(vars.local_wires[self.wire_shift()]);
        for (i, point) in coset.into_iter().enumerate() {
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let computed_value = interpolant.eval_base(point);
            constraints.extend((value - computed_value).to_basefield_array());
        }

        let evaluation_point = vars.get_local_ext_algebra(self.wires_evaluation_point());
        let evaluation_value = vars.get_local_ext_algebra(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval(evaluation_point);
        constraints.extend((evaluation_value - computed_evaluation_value).to_basefield_array());

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        let coeffs = (0..self.num_points())
            .map(|i| vars.get_local_ext(self.wires_coeff(i)))
            .collect();
        let interpolant = PolynomialCoeffs::new(coeffs);

        let coset = self.coset(vars.local_wires[self.wire_shift()]);
        for (i, point) in coset.into_iter().enumerate() {
            let value = vars.get_local_ext(self.wires_value(i));
            let computed_value = interpolant.eval_base(point);
            yield_constr.many((value - computed_value).to_basefield_array());
        }

        let evaluation_point = vars.get_local_ext(self.wires_evaluation_point());
        let evaluation_value = vars.get_local_ext(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval(evaluation_point);
        yield_constr.many((evaluation_value - computed_evaluation_value).to_basefield_array());
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.wires_coeff(i)))
            .collect();
        let interpolant = PolynomialCoeffsExtAlgebraTarget(coeffs);

        let coset = self.coset_ext_circuit(builder, vars.local_wires[self.wire_shift()]);
        for (i, point) in coset.into_iter().enumerate() {
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let computed_value = interpolant.eval_scalar(builder, point);
            constraints.extend(
                builder
                    .sub_ext_algebra(value, computed_value)
                    .to_ext_target_array(),
            );
        }

        let evaluation_point = vars.get_local_ext_algebra(self.wires_evaluation_point());
        let evaluation_value = vars.get_local_ext_algebra(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval(builder, evaluation_point);
        constraints.extend(
            builder
                .sub_ext_algebra(evaluation_value, computed_evaluation_value)
                .to_ext_target_array(),
        );

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = InterpolationGenerator::<F, D> {
            row,
            gate: *self,
            _phantom: PhantomData,
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.end()
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        // The highest power of x is `num_points - 1`, and then multiplication by the coefficient
        // adds 1.
        self.num_points()
    }

    fn num_constraints(&self) -> usize {
        // num_points * D constraints to check for consistency between the coefficients and the
        // point-value pairs, plus D constraints for the evaluation value.
        self.num_points() * D + D
    }
}

#[derive(Debug)]
struct InterpolationGenerator<F: RichField + Extendable<D>, const D: usize> {
    row: usize,
    gate: HighDegreeInterpolationGate<F, D>,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for InterpolationGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |column| {
            Target::Wire(Wire {
                row: self.row,
                column,
            })
        };

        let local_targets = |columns: Range<usize>| columns.map(local_target);

        let num_points = self.gate.num_points();
        let mut deps = Vec::with_capacity(1 + D + num_points * D);

        deps.push(local_target(self.gate.wire_shift()));
        deps.extend(local_targets(self.gate.wires_evaluation_point()));
        for i in 0..num_points {
            deps.extend(local_targets(self.gate.wires_value(i)));
        }
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let get_local_wire = |column| witness.get_wire(local_wire(column));

        let get_local_ext = |wire_range: Range<usize>| {
            debug_assert_eq!(wire_range.len(), D);
            let values = wire_range.map(get_local_wire).collect::<Vec<_>>();
            let arr = values.try_into().unwrap();
            F::Extension::from_basefield_array(arr)
        };

        // Compute the interpolant.
        let points = self.gate.coset(get_local_wire(self.gate.wire_shift()));
        let points = points
            .into_iter()
            .enumerate()
            .map(|(i, point)| (point.into(), get_local_ext(self.gate.wires_value(i))))
            .collect::<Vec<_>>();
        let interpolant = interpolant(&points);

        for (i, &coeff) in interpolant.coeffs.iter().enumerate() {
            let wires = self.gate.wires_coeff(i).map(local_wire);
            out_buffer.set_ext_wires(wires, coeff);
        }

        let evaluation_point = get_local_ext(self.gate.wires_evaluation_point());
        let evaluation_value = interpolant.eval(evaluation_point);
        let evaluation_value_wires = self.gate.wires_evaluation_value().map(local_wire);
        out_buffer.set_ext_wires(evaluation_value_wires, evaluation_value);
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::polynomial::PolynomialCoeffs;
    use plonky2_field::types::Field;

    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::high_degree_interpolation::HighDegreeInterpolationGate;
    use crate::gates::interpolation::InterpolationGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn wire_indices() {
        let gate = HighDegreeInterpolationGate::<GoldilocksField, 4> {
            subgroup_bits: 1,
            _phantom: PhantomData,
        };

        // The exact indices aren't really important, but we want to make sure we don't have any
        // overlaps or gaps.
        assert_eq!(gate.wire_shift(), 0);
        assert_eq!(gate.wires_value(0), 1..5);
        assert_eq!(gate.wires_value(1), 5..9);
        assert_eq!(gate.wires_evaluation_point(), 9..13);
        assert_eq!(gate.wires_evaluation_value(), 13..17);
        assert_eq!(gate.wires_coeff(0), 17..21);
        assert_eq!(gate.wires_coeff(1), 21..25);
        assert_eq!(gate.num_wires(), 25);
    }

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(HighDegreeInterpolationGate::new(2));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(HighDegreeInterpolationGate::new(2))
    }

    #[test]
    fn test_gate_constraint() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;

        /// Returns the local wires for an interpolation gate for given coeffs, points and eval point.
        fn get_wires(
            gate: &HighDegreeInterpolationGate<F, D>,
            shift: F,
            coeffs: PolynomialCoeffs<FF>,
            eval_point: FF,
        ) -> Vec<FF> {
            let points = gate.coset(shift);
            let mut v = vec![shift];
            for x in points {
                v.extend(coeffs.eval(x.into()).0);
            }
            v.extend(eval_point.0);
            v.extend(coeffs.eval(eval_point).0);
            for i in 0..coeffs.len() {
                v.extend(coeffs.coeffs[i].0);
            }
            v.iter().map(|&x| x.into()).collect()
        }

        // Get a working row for InterpolationGate.
        let shift = F::rand();
        let coeffs = PolynomialCoeffs::new(vec![FF::rand(), FF::rand()]);
        let eval_point = FF::rand();
        let gate = HighDegreeInterpolationGate::<F, D>::new(1);
        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(&gate, shift, coeffs, eval_point),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
