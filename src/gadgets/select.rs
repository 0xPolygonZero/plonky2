use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gates::arithmetic::ArithmeticExtensionGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Selects `x` or `y` based on `b`, which is assumed to be binary, i.e., this returns `if b { x } else { y }`.
    /// This expression is gotten as `bx - (by-y)`, which can be computed with a single `ArithmeticExtensionGate`.
    /// Note: This does not range-check `b`.
    pub fn select_ext(
        &mut self,
        b: ExtensionTarget<D>,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        // Holds `by - y`.
        let (gate, range) = if let Some((g, c_0, c_1)) = self.free_arithmetic {
            if c_0 == F::ONE && c_1 == F::NEG_ONE {
                (g, ArithmeticExtensionGate::<D>::wires_third_output())
            } else {
                (
                    self.num_gates(),
                    ArithmeticExtensionGate::<D>::wires_first_output(),
                )
            }
        } else {
            (
                self.num_gates(),
                ArithmeticExtensionGate::<D>::wires_first_output(),
            )
        };
        let first_out = ExtensionTarget::from_range(gate, range);
        self.double_arithmetic_extension(F::ONE, F::NEG_ONE, b, y, y, b, x, first_out)
            .1
    }

    /// See `select_ext`.
    pub fn select(&mut self, b: Target, x: Target, y: Target) -> Target {
        let b_ext = self.convert_to_ext(b);
        let x_ext = self.convert_to_ext(x);
        let y_ext = self.convert_to_ext(y);
        self.select_ext(b_ext, x_ext, y_ext).to_target_array()[0]
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_select() -> Result<()> {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let config = CircuitConfig::large_config();
        let mut pw = PartialWitness::new(config.num_wires);
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let (x, y) = (FF::rand(), FF::rand());
        let xt = builder.add_virtual_extension_target();
        let yt = builder.add_virtual_extension_target();
        let truet = builder.add_virtual_extension_target();
        let falset = builder.add_virtual_extension_target();

        pw.set_extension_target(xt, x);
        pw.set_extension_target(yt, y);
        pw.set_extension_target(truet, FF::ONE);
        pw.set_extension_target(falset, FF::ZERO);

        let should_be_x = builder.select_ext(truet, xt, yt);
        let should_be_y = builder.select_ext(falset, xt, yt);

        builder.assert_equal_extension(should_be_x, xt);
        builder.assert_equal_extension(should_be_y, yt);

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
