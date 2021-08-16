use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gates::arithmetic::ArithmeticExtensionGate;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Selects `x` or `y` based on `b`, i.e., this returns `if b { x } else { y }`.
    pub fn select_ext(
        &mut self,
        b: BoolTarget,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let b_ext = self.convert_to_ext(b.target);
        self.select_ext_generalized(b_ext, x, y)
    }

    /// Like `select_ext`, but accepts a condition input which does not necessarily have to be
    /// binary. In this case, it computes the arithmetic generalization of `if b { x } else { y }`,
    /// i.e. `bx - (by-y)`, which can be computed with a single `ArithmeticExtensionGate`.
    pub fn select_ext_generalized(
        &mut self,
        b: ExtensionTarget<D>,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let tmp = self.mul_sub_extension(b, y, y);
        self.mul_sub_extension(b, x, tmp)
    }

    /// See `select_ext`.
    pub fn select(&mut self, b: BoolTarget, x: Target, y: Target) -> Target {
        let x_ext = self.convert_to_ext(x);
        let y_ext = self.convert_to_ext(y);
        self.select_ext(b, x_ext, y_ext).to_target_array()[0]
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
        let truet = builder._true();
        let falset = builder._false();

        pw.set_extension_target(xt, x);
        pw.set_extension_target(yt, y);

        let should_be_x = builder.select_ext(truet, xt, yt);
        let should_be_y = builder.select_ext(falset, xt, yt);

        builder.assert_equal_extension(should_be_x, xt);
        builder.assert_equal_extension(should_be_y, yt);

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
