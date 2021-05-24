use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::{Extendable, FieldExtension, OEF};
use crate::field::field::Field;
use crate::target::Target;

#[derive(Copy, Clone, Debug)]
pub struct ExtensionTarget<const D: usize>([Target; D]);

impl<F: Field> CircuitBuilder<F> {
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

    pub fn mul_extension<const D: usize>(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        let w = self.constant(F::Extension::W);
        let mut res = [self.zero(); D];
        for i in 0..D {
            for j in 0..D {
                res[(i + j) % D] = if i + j < D {
                    self.mul_add(a.0[i], b.0[j], res[(i + j) % D])
                } else {
                    let tmp = self.mul_add(a.0[i], b.0[j], res[(i + j) % D]);
                    self.mul(w, tmp)
                }
            }
        }
        ExtensionTarget(res)
    }
}
