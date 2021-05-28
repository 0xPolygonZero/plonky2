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

#[derive(Copy, Clone, Debug)]
pub struct ExtensionExtensionTarget<const D: usize>(pub [ExtensionTarget<D>; D]);

impl<const D: usize> ExtensionExtensionTarget<D> {
    pub fn to_ext_target_array(&self) -> [ExtensionTarget<D>; D] {
        self.0
    }
}

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn constant_extension(&mut self, c: F) -> ExtensionTarget<D> {
        let c_parts = c.to_basefield_array();
        let mut parts = [self.zero(); D];
        for i in 0..D {
            parts[i] = self.constant(c_parts[i]);
        }
        ExtensionTarget(parts)
    }

    pub fn zero_extension(&mut self) -> ExtensionTarget<D> {
        self.constant_extension(F::ZERO)
    }

    pub fn one_extension(&mut self) -> ExtensionTarget<D> {
        self.constant_extension(F::ONE)
    }

    pub fn two_extension(&mut self) -> ExtensionTarget<D> {
        self.constant_extension(F::TWO)
    }

    pub fn add_extension(
        &mut self,
        mut a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        for i in 0..D {
            a.0[i] = self.add(a.0[i], b.0[i]);
        }
        a
    }

    pub fn add_many_extension(&mut self, terms: &[ExtensionTarget<D>]) -> ExtensionTarget<D> {
        let mut sum = self.zero_extension();
        for term in terms {
            sum = self.add_extension(sum, *term);
        }
        sum
    }

    pub fn sub_extension(
        &mut self,
        mut a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        for i in 0..D {
            a.0[i] = self.sub(a.0[i], b.0[i]);
        }
        a
    }

    pub fn sub_ext_ext(
        &mut self,
        mut a: ExtensionExtensionTarget<D>,
        b: ExtensionExtensionTarget<D>,
    ) -> ExtensionExtensionTarget<D> {
        for i in 0..D {
            a.0[i] = self.sub_extension(a.0[i], b.0[i]);
        }
        a
    }

    pub fn mul_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let mut res = [self.zero(); D];
        for i in 0..D {
            for j in 0..D {
                res[(i + j) % D] = if i + j < D {
                    self.mul_add(a.0[i], b.0[j], res[(i + j) % D])
                } else {
                    // W * a[i] * b[i] + res[(i + j) % D]
                    self.arithmetic(F::Extension::W, a.0[i], b.0[i], F::ONE, res[(i + j) % D])
                }
            }
        }
        ExtensionTarget(res)
    }

    pub fn mul_many_extension(&mut self, terms: &[ExtensionTarget<D>]) -> ExtensionTarget<D> {
        let mut product = self.one_extension();
        for term in terms {
            product = self.mul_extension(product, *term);
        }
        product
    }

    // TODO: Not sure if we should use this long term. It's just convenient during the switch to EF.
    pub fn mul_add_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let product = self.mul_extension(a, b);
        self.add_extension(product, c)
    }

    /// Returns `a * b`, where `b` is in the extension field and `a` is in the base field.
    pub fn scalar_mul(&mut self, a: Target, mut b: ExtensionTarget<D>) -> ExtensionTarget<D> {
        for i in 0..D {
            b.0[i] = self.mul(a, b.0[i]);
        }
        b
    }
}
