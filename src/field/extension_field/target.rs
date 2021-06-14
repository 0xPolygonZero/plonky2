use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::algebra::ExtensionAlgebra;
use crate::field::extension_field::{Extendable, FieldExtension, OEF};
use crate::field::field::Field;
use crate::gates::mul_extension::MulExtensionGate;
use crate::target::Target;
use num::traits::real::Real;
use std::convert::{TryFrom, TryInto};
use std::ops::Range;

/// `Target`s representing an element of an extension field.
#[derive(Copy, Clone, Debug)]
pub struct ExtensionTarget<const D: usize>(pub [Target; D]);

impl<const D: usize> ExtensionTarget<D> {
    pub fn to_target_array(&self) -> [Target; D] {
        self.0
    }

    pub fn frobenius<F: Extendable<D>>(&self, builder: &mut CircuitBuilder<F, D>) -> Self {
        let arr = self.to_target_array();
        let k = (F::ORDER - 1) / (D as u64);
        let zs = (0..D as u64)
            .map(|i| builder.constant(F::Extension::W.exp(k * i)))
            .collect::<Vec<_>>();

        let mut res = Vec::with_capacity(D);
        for (z, a) in zs.into_iter().zip(arr) {
            res.push(builder.mul(z, a));
        }

        res.try_into().unwrap()
    }

    pub fn repeated_frobenius<F: Extendable<D>>(
        &self,
        count: usize,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        if count == 0 {
            return *self;
        } else if count >= D {
            return self.repeated_frobenius(count % D, builder);
        }
        let arr = self.to_target_array();
        let k = (F::ORDER - 1) / (D as u64);
        let z0 = F::W.exp(k * count as u64);
        let zs = z0
            .powers()
            .take(D)
            .map(|z| builder.constant(z))
            .collect::<Vec<_>>();

        let mut res = Vec::with_capacity(D);
        for (z, a) in zs.into_iter().zip(arr) {
            res.push(builder.mul(z, a));
        }

        res.try_into().unwrap()
    }

    pub fn from_range(gate: usize, range: Range<usize>) -> Self {
        debug_assert_eq!(range.end - range.start, D);
        Target::wires_from_range(gate, range).try_into().unwrap()
    }
}

impl<const D: usize> TryFrom<Vec<Target>> for ExtensionTarget<D> {
    type Error = Vec<Target>;

    fn try_from(value: Vec<Target>) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

/// `Target`s representing an element of an extension of an extension field.
#[derive(Copy, Clone, Debug)]
pub struct ExtensionAlgebraTarget<const D: usize>(pub [ExtensionTarget<D>; D]);

impl<const D: usize> ExtensionAlgebraTarget<D> {
    pub fn to_ext_target_array(&self) -> [ExtensionTarget<D>; D] {
        self.0
    }
}

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn constant_extension(&mut self, c: F::Extension) -> ExtensionTarget<D> {
        let c_parts = c.to_basefield_array();
        let mut parts = [self.zero(); D];
        for i in 0..D {
            parts[i] = self.constant(c_parts[i]);
        }
        ExtensionTarget(parts)
    }

    pub fn constant_ext_algebra(
        &mut self,
        c: ExtensionAlgebra<F::Extension, D>,
    ) -> ExtensionAlgebraTarget<D> {
        let c_parts = c.to_basefield_array();
        let mut parts = [self.zero_extension(); D];
        for i in 0..D {
            parts[i] = self.constant_extension(c_parts[i]);
        }
        ExtensionAlgebraTarget(parts)
    }

    pub fn zero_extension(&mut self) -> ExtensionTarget<D> {
        self.constant_extension(F::Extension::ZERO)
    }

    pub fn one_extension(&mut self) -> ExtensionTarget<D> {
        self.constant_extension(F::Extension::ONE)
    }

    pub fn two_extension(&mut self) -> ExtensionTarget<D> {
        self.constant_extension(F::Extension::TWO)
    }

    pub fn zero_ext_algebra(&mut self) -> ExtensionAlgebraTarget<D> {
        self.constant_ext_algebra(ExtensionAlgebra::ZERO)
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

    pub fn add_ext_algebra(
        &mut self,
        mut a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        for i in 0..D {
            a.0[i] = self.add_extension(a.0[i], b.0[i]);
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

    pub fn sub_ext_algebra(
        &mut self,
        mut a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        for i in 0..D {
            a.0[i] = self.sub_extension(a.0[i], b.0[i]);
        }
        a
    }

    pub fn mul_extension_with_const(
        &mut self,
        const_0: F,
        multiplicand_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let gate = self.add_gate(MulExtensionGate::new(), vec![const_0]);

        let wire_multiplicand_0 =
            ExtensionTarget::from_range(gate, MulExtensionGate::<D>::wires_multiplicand_0());
        let wire_multiplicand_1 =
            ExtensionTarget::from_range(gate, MulExtensionGate::<D>::wires_multiplicand_1());
        let wire_output = ExtensionTarget::from_range(gate, MulExtensionGate::<D>::wires_output());

        self.route_extension(multiplicand_0, wire_multiplicand_0);
        self.route_extension(multiplicand_1, wire_multiplicand_1);
        wire_output
    }

    pub fn mul_extension(
        &mut self,
        multiplicand_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        self.mul_extension_with_const(F::ONE, multiplicand_0, multiplicand_1)
    }

    pub fn mul_ext_algebra(
        &mut self,
        a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        let mut res = [self.zero_extension(); D];
        let w = self.constant(F::Extension::W);
        for i in 0..D {
            for j in 0..D {
                let ai_bi = self.mul_extension(a.0[i], b.0[j]);
                res[(i + j) % D] = if i + j < D {
                    self.add_extension(ai_bi, res[(i + j) % D])
                } else {
                    let w_ai_bi = self.scalar_mul_ext(w, ai_bi);
                    self.add_extension(w_ai_bi, res[(i + j) % D])
                }
            }
        }
        ExtensionAlgebraTarget(res)
    }

    pub fn mul_many_extension(&mut self, terms: &[ExtensionTarget<D>]) -> ExtensionTarget<D> {
        let mut product = self.one_extension();
        for term in terms {
            product = self.mul_extension(product, *term);
        }
        product
    }

    /// Like `mul_add`, but for `ExtensionTarget`s. Note that, unlike `mul_add`, this has no
    /// performance benefit over separate muls and adds.
    pub fn mul_add_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let product = self.mul_extension(a, b);
        self.add_extension(product, c)
    }

    /// Like `mul_sub`, but for `ExtensionTarget`s. Note that, unlike `mul_sub`, this has no
    /// performance benefit over separate muls and subs.
    pub fn scalar_mul_sub_extension(
        &mut self,
        a: Target,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let product = self.scalar_mul_ext(a, b);
        self.sub_extension(product, c)
    }

    /// Returns `a * b`, where `b` is in the extension field and `a` is in the base field.
    pub fn scalar_mul_ext(&mut self, a: Target, mut b: ExtensionTarget<D>) -> ExtensionTarget<D> {
        let a_ext = self.convert_to_ext(a);
        self.mul_extension(a_ext, b)
    }

    /// Returns `a * b`, where `b` is in the extension of the extension field, and `a` is in the
    /// extension field.
    pub fn scalar_mul_ext_algebra(
        &mut self,
        a: ExtensionTarget<D>,
        mut b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        for i in 0..D {
            b.0[i] = self.mul_extension(a, b.0[i]);
        }
        b
    }

    pub fn convert_to_ext(&mut self, t: Target) -> ExtensionTarget<D> {
        let zero = self.zero();
        let mut arr = [zero; D];
        arr[0] = t;
        ExtensionTarget(arr)
    }
}

/// Flatten the slice by sending every extension target to its D-sized canonical representation.
pub fn flatten_target<const D: usize>(l: &[ExtensionTarget<D>]) -> Vec<Target> {
    l.iter()
        .flat_map(|x| x.to_target_array().to_vec())
        .collect()
}

/// Batch every D-sized chunks into extension targets.
pub fn unflatten_target<F: Extendable<D>, const D: usize>(l: &[Target]) -> Vec<ExtensionTarget<D>> {
    debug_assert_eq!(l.len() % D, 0);
    l.chunks_exact(D)
        .map(|c| c.to_vec().try_into().unwrap())
        .collect()
}
