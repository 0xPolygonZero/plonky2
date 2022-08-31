use std::marker::PhantomData;
use std::ops::Range;

use plonky2_field::extension::algebra::PolynomialCoeffsAlgebra;
use plonky2_field::extension::{Extendable, FieldExtension};
use plonky2_field::interpolation::interpolant;
use plonky2_field::polynomial::PolynomialCoeffs;
use plonky2_field::types::Field;

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

/// One of the instantiations of `InterpolationGate`: all constraints are degree <= 2.
/// The lower degree is a tradeoff for more gates (`eval_unfiltered_recursively` for
/// this version uses more gates than `LowDegreeInterpolationGate`).
#[derive(Copy, Clone, Debug)]
pub struct LowDegreeInterpolationGate<F: RichField + Extendable<D>, const D: usize> {
    pub subgroup_bits: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> InterpolationGate<F, D>
    for LowDegreeInterpolationGate<F, D>
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

impl<F: RichField + Extendable<D>, const D: usize> LowDegreeInterpolationGate<F, D> {
    /// `powers_shift(i)` is the wire index of `wire_shift^i`.
    pub fn powers_shift(&self, i: usize) -> usize {
        debug_assert!(0 < i && i < self.num_points());
        if i == 1 {
            return self.wire_shift();
        }
        self.end_coeffs() + i - 2
    }

    /// `powers_evalutation_point(i)` is the wire index of `evalutation_point^i`.
    pub fn powers_evaluation_point(&self, i: usize) -> Range<usize> {
        debug_assert!(0 < i && i < self.num_points());
        if i == 1 {
            return self.wires_evaluation_point();
        }
        let start = self.end_coeffs() + self.num_points() - 2 + (i - 2) * D;
        start..start + D
    }

    /// End of wire indices, exclusive.
    fn end(&self) -> usize {
        self.powers_evaluation_point(self.num_points() - 1).end
    }

    /// The domain of the points we're interpolating.
    fn coset(&self, shift: F) -> impl Iterator<Item = F> {
        let g = F::primitive_root_of_unity(self.subgroup_bits);
        let size = 1 << self.subgroup_bits;
        // Speed matters here, so we avoid `cyclic_subgroup_coset_known_order` which allocates.
        g.powers().take(size).map(move |x| x * shift)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for LowDegreeInterpolationGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<D={}>", self, D)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.wires_coeff(i)))
            .collect::<Vec<_>>();

        let mut powers_shift = (1..self.num_points())
            .map(|i| vars.local_wires[self.powers_shift(i)])
            .collect::<Vec<_>>();
        let shift = powers_shift[0];
        for i in 1..self.num_points() - 1 {
            constraints.push(powers_shift[i - 1] * shift - powers_shift[i]);
        }
        powers_shift.insert(0, F::Extension::ONE);
        // `altered_coeffs[i] = c_i * shift^i`, where `c_i` is the original coefficient.
        // Then, `altered(w^i) = original(shift*w^i)`.
        let altered_coeffs = coeffs
            .iter()
            .zip(powers_shift)
            .map(|(&c, p)| c.scalar_mul(p))
            .collect::<Vec<_>>();
        let interpolant = PolynomialCoeffsAlgebra::new(coeffs);
        let altered_interpolant = PolynomialCoeffsAlgebra::new(altered_coeffs);

        for (i, point) in F::Extension::two_adic_subgroup(self.subgroup_bits)
            .into_iter()
            .enumerate()
        {
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let computed_value = altered_interpolant.eval_base(point);
            constraints.extend((value - computed_value).to_basefield_array());
        }

        let evaluation_point_powers = (1..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.powers_evaluation_point(i)))
            .collect::<Vec<_>>();
        let evaluation_point = evaluation_point_powers[0];
        for i in 1..self.num_points() - 1 {
            constraints.extend(
                (evaluation_point_powers[i - 1] * evaluation_point - evaluation_point_powers[i])
                    .to_basefield_array(),
            );
        }
        let evaluation_value = vars.get_local_ext_algebra(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval_with_powers(&evaluation_point_powers);
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
            .collect::<Vec<_>>();

        let mut powers_shift = (1..self.num_points())
            .map(|i| vars.local_wires[self.powers_shift(i)])
            .collect::<Vec<_>>();
        let shift = powers_shift[0];
        for i in 1..self.num_points() - 1 {
            yield_constr.one(powers_shift[i - 1] * shift - powers_shift[i]);
        }
        powers_shift.insert(0, F::ONE);
        // `altered_coeffs[i] = c_i * shift^i`, where `c_i` is the original coefficient.
        // Then, `altered(w^i) = original(shift*w^i)`.
        let altered_coeffs = coeffs
            .iter()
            .zip(powers_shift)
            .map(|(&c, p)| c.scalar_mul(p))
            .collect::<Vec<_>>();
        let interpolant = PolynomialCoeffs::new(coeffs);
        let altered_interpolant = PolynomialCoeffs::new(altered_coeffs);

        for (i, point) in F::two_adic_subgroup(self.subgroup_bits)
            .into_iter()
            .enumerate()
        {
            let value = vars.get_local_ext(self.wires_value(i));
            let computed_value = altered_interpolant.eval_base(point);
            yield_constr.many((value - computed_value).to_basefield_array());
        }

        let evaluation_point_powers = (1..self.num_points())
            .map(|i| vars.get_local_ext(self.powers_evaluation_point(i)))
            .collect::<Vec<_>>();
        let evaluation_point = evaluation_point_powers[0];
        for i in 1..self.num_points() - 1 {
            yield_constr.many(
                (evaluation_point_powers[i - 1] * evaluation_point - evaluation_point_powers[i])
                    .to_basefield_array(),
            );
        }
        let evaluation_value = vars.get_local_ext(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval_with_powers(&evaluation_point_powers);
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
            .collect::<Vec<_>>();

        let mut powers_shift = (1..self.num_points())
            .map(|i| vars.local_wires[self.powers_shift(i)])
            .collect::<Vec<_>>();
        let shift = powers_shift[0];
        for i in 1..self.num_points() - 1 {
            constraints.push(builder.mul_sub_extension(
                powers_shift[i - 1],
                shift,
                powers_shift[i],
            ));
        }
        powers_shift.insert(0, builder.one_extension());
        // `altered_coeffs[i] = c_i * shift^i`, where `c_i` is the original coefficient.
        // Then, `altered(w^i) = original(shift*w^i)`.
        let altered_coeffs = coeffs
            .iter()
            .zip(powers_shift)
            .map(|(&c, p)| builder.scalar_mul_ext_algebra(p, c))
            .collect::<Vec<_>>();
        let interpolant = PolynomialCoeffsExtAlgebraTarget(coeffs);
        let altered_interpolant = PolynomialCoeffsExtAlgebraTarget(altered_coeffs);

        for (i, point) in F::Extension::two_adic_subgroup(self.subgroup_bits)
            .into_iter()
            .enumerate()
        {
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let point = builder.constant_extension(point);
            let computed_value = altered_interpolant.eval_scalar(builder, point);
            constraints.extend(
                builder
                    .sub_ext_algebra(value, computed_value)
                    .to_ext_target_array(),
            );
        }

        let evaluation_point_powers = (1..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.powers_evaluation_point(i)))
            .collect::<Vec<_>>();
        let evaluation_point = evaluation_point_powers[0];
        for i in 1..self.num_points() - 1 {
            let neg_one_ext = builder.neg_one_extension();
            let neg_new_power =
                builder.scalar_mul_ext_algebra(neg_one_ext, evaluation_point_powers[i]);
            let constraint = builder.mul_add_ext_algebra(
                evaluation_point,
                evaluation_point_powers[i - 1],
                neg_new_power,
            );
            constraints.extend(constraint.to_ext_target_array());
        }
        let evaluation_value = vars.get_local_ext_algebra(self.wires_evaluation_value());
        let computed_evaluation_value =
            interpolant.eval_with_powers(builder, &evaluation_point_powers);
        // let evaluation_point = vars.get_local_ext_algebra(self.wires_evaluation_point());
        // let evaluation_value = vars.get_local_ext_algebra(self.wires_evaluation_value());
        // let computed_evaluation_value = interpolant.eval(builder, evaluation_point);
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
        2
    }

    fn num_constraints(&self) -> usize {
        // `num_points * D` constraints to check for consistency between the coefficients and the
        // point-value pairs, plus `D` constraints for the evaluation value, plus `(D+1)*(num_points-2)`
        // to check power constraints for evaluation point and shift.
        self.num_points() * D + D + (D + 1) * (self.num_points() - 2)
    }
}

#[derive(Debug)]
struct InterpolationGenerator<F: RichField + Extendable<D>, const D: usize> {
    row: usize,
    gate: LowDegreeInterpolationGate<F, D>,
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

        let wire_shift = get_local_wire(self.gate.wire_shift());

        for (i, power) in wire_shift
            .powers()
            .take(self.gate.num_points())
            .enumerate()
            .skip(2)
        {
            out_buffer.set_wire(local_wire(self.gate.powers_shift(i)), power);
        }

        // Compute the interpolant.
        let points = self.gate.coset(wire_shift);
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
        for (i, power) in evaluation_point
            .powers()
            .take(self.gate.num_points())
            .enumerate()
            .skip(2)
        {
            out_buffer.set_extension_target(
                ExtensionTarget::from_range(self.row, self.gate.powers_evaluation_point(i)),
                power,
            );
        }
        let evaluation_value = interpolant.eval(evaluation_point);
        let evaluation_value_wires = self.gate.wires_evaluation_value().map(local_wire);
        out_buffer.set_ext_wires(evaluation_value_wires, evaluation_value);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::extension::quadratic::QuadraticExtension;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::polynomial::PolynomialCoeffs;
    use plonky2_field::types::Field;

    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::interpolation::InterpolationGate;
    use crate::gates::low_degree_interpolation::LowDegreeInterpolationGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(LowDegreeInterpolationGate::new(4));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(LowDegreeInterpolationGate::new(4))
    }

    #[test]
    fn test_gate_constraint() {
        type F = GoldilocksField;
        type FF = QuadraticExtension<GoldilocksField>;
        const D: usize = 2;

        /// Returns the local wires for an interpolation gate for given coeffs, points and eval point.
        fn get_wires(
            gate: &LowDegreeInterpolationGate<F, D>,
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
            v.extend(shift.powers().skip(2).take(gate.num_points() - 2));
            v.extend(
                eval_point
                    .powers()
                    .skip(2)
                    .take(gate.num_points() - 2)
                    .flat_map(|ff| ff.0),
            );
            v.iter().map(|&x| x.into()).collect()
        }

        // Get a working row for LowDegreeInterpolationGate.
        let subgroup_bits = 4;
        let shift = F::rand();
        let coeffs = PolynomialCoeffs::new(FF::rand_vec(1 << subgroup_bits));
        let eval_point = FF::rand();
        let gate = LowDegreeInterpolationGate::<F, D>::new(subgroup_bits);
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
