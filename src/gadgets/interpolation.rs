use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::interpolation::InterpolationGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Interpolates a polynomial, whose points are a coset of the multiplicative subgroup with the
    /// given size, and whose values are given. Returns the evaluation of the interpolant at
    /// `evaluation_point`.
    pub fn interpolate_coset(
        &mut self,
        subgroup_bits: usize,
        coset_shift: Target,
        values: &[ExtensionTarget<D>],
        evaluation_point: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let gate = InterpolationGate::new(subgroup_bits);
        let gate_index = self.add_gate(gate.clone(), vec![]);
        self.connect(coset_shift, Target::wire(gate_index, gate.wire_shift()));
        for (i, &v) in values.iter().enumerate() {
            self.connect_extension(
                v,
                ExtensionTarget::from_range(gate_index, gate.wires_value(i)),
            );
        }
        self.connect_extension(
            evaluation_point,
            ExtensionTarget::from_range(gate_index, gate.wires_evaluation_point()),
        );

        ExtensionTarget::from_range(gate_index, gate.wires_evaluation_value())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::extension_field::FieldExtension;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::interpolation::interpolant;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_interpolate() -> Result<()> {
        type F = GoldilocksField;
        type FF = QuarticExtension<GoldilocksField>;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let subgroup_bits = 2;
        let len = 1 << subgroup_bits;
        let coset_shift = F::rand();
        let g = F::primitive_root_of_unity(subgroup_bits);
        let points = F::cyclic_subgroup_coset_known_order(g, coset_shift, len);
        let values = FF::rand_vec(len);

        let homogeneous_points = points
            .iter()
            .zip(values.iter())
            .map(|(&a, &b)| (<FF as FieldExtension<4>>::from_basefield(a), b))
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

        let eval = builder.interpolate_coset(subgroup_bits, coset_shift_target, &value_targets, zt);
        let true_eval_target = builder.constant_extension(true_eval);
        builder.connect_extension(eval, true_eval_target);

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
