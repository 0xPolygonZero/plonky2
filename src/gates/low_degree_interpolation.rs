use std::marker::PhantomData;
use std::ops::Range;

use crate::field::extension_field::algebra::PolynomialCoeffsAlgebra;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::{Field, RichField};
use crate::field::interpolation::interpolant;
use crate::gadgets::interpolation::InterpolationGate;
use crate::gadgets::polynomial::PolynomialCoeffsExtAlgebraTarget;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::polynomial::polynomial::PolynomialCoeffs;

/// Interpolates a polynomial, whose points are a (base field) coset of the multiplicative subgroup
/// with the given size, and whose values are extension field elements, given by input wires.
/// Outputs the evaluation of the interpolant at a given (extension field) evaluation point.
#[derive(Copy, Clone, Debug)]
pub(crate) struct LowDegreeInterpolationGate<F: RichField + Extendable<D>, const D: usize> {
    pub subgroup_bits: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> LowDegreeInterpolationGate<F, D> {
    pub fn powers_init(&self, i: usize) -> usize {
        debug_assert!(0 < i && i < self.num_points());
        if i == 1 {
            return self.wire_shift();
        }
        self.end_coeffs() + i - 2
    }

    pub fn powers_eval(&self, i: usize) -> Range<usize> {
        debug_assert!(0 < i && i < self.num_points());
        if i == 1 {
            return self.wires_evaluation_point();
        }
        let start = self.end_coeffs() + self.num_points() - 2 + (i - 2) * D;
        start..start + D
    }

    /// End of wire indices, exclusive.
    fn end(&self) -> usize {
        self.powers_eval(self.num_points() - 1).end
    }

    /// The domain of the points we're interpolating.
    fn coset(&self, shift: F) -> impl Iterator<Item = F> {
        let g = F::primitive_root_of_unity(self.subgroup_bits);
        let size = 1 << self.subgroup_bits;
        // Speed matters here, so we avoid `cyclic_subgroup_coset_known_order` which allocates.
        g.powers().take(size).map(move |x| x * shift)
    }
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

    /// Wire index of the coset shift.
    fn wire_shift(&self) -> usize {
        0
    }

    fn start_values(&self) -> usize {
        1
    }

    /// Wire indices of the `i`th interpolant value.
    fn wires_value(&self, i: usize) -> Range<usize> {
        debug_assert!(i < self.num_points());
        let start = self.start_values() + i * D;
        start..start + D
    }

    fn start_evaluation_point(&self) -> usize {
        self.start_values() + self.num_points() * D
    }

    /// Wire indices of the point to evaluate the interpolant at.
    fn wires_evaluation_point(&self) -> Range<usize> {
        let start = self.start_evaluation_point();
        start..start + D
    }

    fn start_evaluation_value(&self) -> usize {
        self.start_evaluation_point() + D
    }

    /// Wire indices of the interpolated value.
    fn wires_evaluation_value(&self) -> Range<usize> {
        let start = self.start_evaluation_value();
        start..start + D
    }

    fn start_coeffs(&self) -> usize {
        self.start_evaluation_value() + D
    }

    /// The number of routed wires required in the typical usage of this gate, where the points to
    /// interpolate, the evaluation point, and the corresponding value are all routed.
    fn num_routed_wires(&self) -> usize {
        self.start_coeffs()
    }

    /// Wire indices of the interpolant's `i`th coefficient.
    fn wires_coeff(&self, i: usize) -> Range<usize> {
        debug_assert!(i < self.num_points());
        let start = self.start_coeffs() + i * D;
        start..start + D
    }

    fn end_coeffs(&self) -> usize {
        self.start_coeffs() + D * self.num_points()
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

        let mut powers_init = (1..self.num_points())
            .map(|i| vars.local_wires[self.powers_init(i)])
            .collect::<Vec<_>>();
        powers_init.insert(0, F::Extension::ONE);
        let wire_shift = powers_init[1];
        for i in 2..self.num_points() {
            constraints.push(powers_init[i - 1] * wire_shift - powers_init[i]);
        }
        let ocoeffs = coeffs
            .iter()
            .zip(powers_init)
            .map(|(&c, p)| c.scalar_mul(p))
            .collect::<Vec<_>>();
        let interpolant = PolynomialCoeffsAlgebra::new(coeffs);
        let ointerpolant = PolynomialCoeffsAlgebra::new(ocoeffs);

        for (i, point) in F::Extension::two_adic_subgroup(self.subgroup_bits)
            .into_iter()
            .enumerate()
        {
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let computed_value = ointerpolant.eval_base(point);
            constraints.extend(&(value - computed_value).to_basefield_array());
        }

        let evaluation_point_powers = (1..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.powers_eval(i)))
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
        constraints.extend(&(evaluation_value - computed_evaluation_value).to_basefield_array());

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points())
            .map(|i| vars.get_local_ext(self.wires_coeff(i)))
            .collect::<Vec<_>>();
        let mut powers_init = (1..self.num_points())
            .map(|i| vars.local_wires[self.powers_init(i)])
            .collect::<Vec<_>>();
        powers_init.insert(0, F::ONE);
        let wire_shift = powers_init[1];
        for i in 2..self.num_points() {
            constraints.push(powers_init[i - 1] * wire_shift - powers_init[i]);
        }
        let ocoeffs = coeffs
            .iter()
            .zip(powers_init)
            .map(|(&c, p)| c.scalar_mul(p))
            .collect::<Vec<_>>();
        let interpolant = PolynomialCoeffs::new(coeffs);
        let ointerpolant = PolynomialCoeffs::new(ocoeffs);

        for (i, point) in F::two_adic_subgroup(self.subgroup_bits)
            .into_iter()
            .enumerate()
        {
            let value = vars.get_local_ext(self.wires_value(i));
            let computed_value = ointerpolant.eval_base(point);
            constraints.extend(&(value - computed_value).to_basefield_array());
        }

        let evaluation_point_powers = (1..self.num_points())
            .map(|i| vars.get_local_ext(self.powers_eval(i)))
            .collect::<Vec<_>>();
        let evaluation_point = evaluation_point_powers[0];
        for i in 1..self.num_points() - 1 {
            constraints.extend(
                (evaluation_point_powers[i - 1] * evaluation_point - evaluation_point_powers[i])
                    .to_basefield_array(),
            );
        }
        let evaluation_value = vars.get_local_ext(self.wires_evaluation_value());
        let computed_evaluation_value = interpolant.eval_with_powers(&evaluation_point_powers);
        constraints.extend(&(evaluation_value - computed_evaluation_value).to_basefield_array());

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let coeffs = (0..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.wires_coeff(i)))
            .collect::<Vec<_>>();
        let mut powers_init = (1..self.num_points())
            .map(|i| vars.local_wires[self.powers_init(i)])
            .collect::<Vec<_>>();
        powers_init.insert(0, builder.one_extension());
        let wire_shift = powers_init[1];
        for i in 2..self.num_points() {
            constraints.push(builder.mul_sub_extension(
                powers_init[i - 1],
                wire_shift,
                powers_init[i],
            ));
        }
        let ocoeffs = coeffs
            .iter()
            .zip(powers_init)
            .map(|(&c, p)| builder.scalar_mul_ext_algebra(p, c))
            .collect::<Vec<_>>();
        let interpolant = PolynomialCoeffsExtAlgebraTarget(coeffs);
        let ointerpolant = PolynomialCoeffsExtAlgebraTarget(ocoeffs);

        for (i, point) in F::Extension::two_adic_subgroup(self.subgroup_bits)
            .into_iter()
            .enumerate()
        {
            let value = vars.get_local_ext_algebra(self.wires_value(i));
            let point = builder.constant_extension(point);
            let computed_value = ointerpolant.eval_scalar(builder, point);
            constraints.extend(
                &builder
                    .sub_ext_algebra(value, computed_value)
                    .to_ext_target_array(),
            );
        }

        let evaluation_point_powers = (1..self.num_points())
            .map(|i| vars.get_local_ext_algebra(self.powers_eval(i)))
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
            &builder
                .sub_ext_algebra(evaluation_value, computed_evaluation_value)
                .to_ext_target_array(),
        );

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = InterpolationGenerator::<F, D> {
            gate_index,
            gate: self.clone(),
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
        2
    }

    fn num_constraints(&self) -> usize {
        // `num_points * D` constraints to check for consistency between the coefficients and the
        // point-value pairs, plus `D` constraints for the evaluation value, plus `(D+1)*(num_points-2)`
        // to check power constraints for evaluation point and wire shift.
        self.num_points() * D + D + (D + 1) * (self.num_points() - 2)
    }
}

#[derive(Debug)]
struct InterpolationGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    gate: LowDegreeInterpolationGate<F, D>,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for InterpolationGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| {
            Target::Wire(Wire {
                gate: self.gate_index,
                input,
            })
        };

        let local_targets = |inputs: Range<usize>| inputs.map(local_target);

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
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

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
            out_buffer.set_wire(local_wire(self.gate.powers_init(i)), power);
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
        for i in 2..self.gate.num_points() {
            out_buffer.set_extension_target(
                ExtensionTarget::from_range(self.gate_index, self.gate.powers_eval(i)),
                evaluation_point.exp_u64(i as u64),
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

    use crate::field::extension_field::quadratic::QuadraticExtension;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gadgets::interpolation::InterpolationGate;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::low_degree_interpolation::LowDegreeInterpolationGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::vars::EvaluationVars;
    use crate::polynomial::polynomial::PolynomialCoeffs;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(LowDegreeInterpolationGate::new(4));
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<GoldilocksField, _, 4>(LowDegreeInterpolationGate::new(4))
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
            v.iter().map(|&x| x.into()).collect::<Vec<_>>()
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
