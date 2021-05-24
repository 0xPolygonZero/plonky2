use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::{Extendable, FieldExtension, OEF};
use crate::field::field::Field;
use crate::target::Target;

#[derive(Copy, Clone, Debug)]
pub struct ExtensionTarget<const D: usize>(pub [Target; D]);

impl<const D: usize> ExtensionTarget<D> {
    pub fn to_target_array(&self) -> [Target; D] {
        self.0
    }
}

impl<F: Field> CircuitBuilder<F> {
    pub fn zero_extension<const D: usize>(&mut self) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        ExtensionTarget([self.zero(); D])
    }

    pub fn add_extension<const D: usize>(
        &mut self,
        mut a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        for i in 0..D {
            a.0[i] = self.add(a.0[i], b.0[i]);
        }
        a
    }

    pub fn sub_extension<const D: usize>(
        &mut self,
        mut a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        for i in 0..D {
            a.0[i] = self.sub(a.0[i], b.0[i]);
        }
        a
    }

    pub fn mul_extension<const D: usize>(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        let mut res = [self.zero(); D];
        for i in 0..D {
            for j in 0..D {
                res[(i + j) % D] = if i + j < D {
                    self.mul_add(a.0[i], b.0[j], res[(i + j) % D])
                } else {
                    // W * a[i] * b[i] + res[(i + j) % W]
                    self.arithmetic(F::Extension::W, a.0[i], b.0[i], F::Extension::ONE, res[(i + j) % D]);
                }
            }
        }
        ExtensionTarget(res)
    }

    /// Returns a*b where `b` is in the extension field and `a` is in the base field.
    pub fn scalar_mul<const D: usize>(
        &mut self,
        a: Target,
        mut b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        for i in 0..D {
            b.0[i] = self.mul(a, b.0[i]);
        }
        b
    }
}
