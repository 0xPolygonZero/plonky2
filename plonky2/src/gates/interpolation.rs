use std::ops::Range;

use plonky2_field::extension::Extendable;

use crate::gates::gate::Gate;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

/// Trait for gates which interpolate a polynomial, whose points are a (base field) coset of the multiplicative subgroup
/// with the given size, and whose values are extension field elements, given by input wires.
/// Outputs the evaluation of the interpolant at a given (extension field) evaluation point.
pub(crate) trait InterpolationGate<F: RichField + Extendable<D>, const D: usize>:
    Gate<F, D> + Copy
{
    fn new(subgroup_bits: usize) -> Self;

    fn num_points(&self) -> usize;

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

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Interpolates a polynomial, whose points are a coset of the multiplicative subgroup with the
    /// given size, and whose values are given. Returns the evaluation of the interpolant at
    /// `evaluation_point`.
    pub(crate) fn interpolate_coset<G: InterpolationGate<F, D>>(
        &mut self,
        subgroup_bits: usize,
        coset_shift: Target,
        values: &[ExtensionTarget<D>],
        evaluation_point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let gate = G::new(subgroup_bits);
        let row = self.add_gate(gate, vec![]);
        self.connect(coset_shift, Target::wire(row, gate.wire_shift()));
        for (i, &v) in values.iter().enumerate() {
            self.connect_extension(v, ExtensionTarget::from_range(row, gate.wires_value(i)));
        }
        self.connect_extension(
            evaluation_point,
            ExtensionTarget::from_range(row, gate.wires_evaluation_point()),
        );

        ExtensionTarget::from_range(row, gate.wires_evaluation_value())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::extension::FieldExtension;
    use plonky2_field::interpolation::interpolant;
    use plonky2_field::types::Field;

    use crate::gates::high_degree_interpolation::HighDegreeInterpolationGate;
    use crate::gates::low_degree_interpolation::LowDegreeInterpolationGate;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_interpolate() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let subgroup_bits = 2;
        let len = 1 << subgroup_bits;
        let coset_shift = F::rand();
        let g = F::primitive_root_of_unity(subgroup_bits);
        let points = F::cyclic_subgroup_coset_known_order(g, coset_shift, len);
        let values = FF::rand_vec(len);

        let homogeneous_points = points
            .iter()
            .zip(values.iter())
            .map(|(&a, &b)| (<FF as FieldExtension<D>>::from_basefield(a), b))
            .collect::<Vec<_>>();

        let true_interpolant = interpolant(&homogeneous_points);

        let z = FF::rand();
        let true_eval = true_interpolant.eval(z);

        let coset_shift_target = builder.constant(coset_shift);

        let value_targets = values
            .iter()
            .map(|&v| (builder.constant_extension(v)))
            .collect::<Vec<_>>();

        let zt = builder.constant_extension(z);

        let eval_hd = builder.interpolate_coset::<HighDegreeInterpolationGate<F, D>>(
            subgroup_bits,
            coset_shift_target,
            &value_targets,
            zt,
        );
        let eval_ld = builder.interpolate_coset::<LowDegreeInterpolationGate<F, D>>(
            subgroup_bits,
            coset_shift_target,
            &value_targets,
            zt,
        );
        let true_eval_target = builder.constant_extension(true_eval);
        builder.connect_extension(eval_hd, true_eval_target);
        builder.connect_extension(eval_ld, true_eval_target);

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
