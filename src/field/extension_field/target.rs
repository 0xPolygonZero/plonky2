use std::convert::{TryFrom, TryInto};
use std::ops::Range;

use itertools::Itertools;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::algebra::ExtensionAlgebra;
use crate::field::extension_field::{Extendable, FieldExtension, OEF};
use crate::field::field::Field;
use crate::gates::arithmetic::ArithmeticExtensionGate;
use crate::target::Target;

/// `Target`s representing an element of an extension field.
#[derive(Copy, Clone, Debug)]
pub struct ExtensionTarget<const D: usize>(pub [Target; D]);

impl<const D: usize> ExtensionTarget<D> {
    pub fn to_target_array(&self) -> [Target; D] {
        self.0
    }

    pub fn frobenius<F: Extendable<D>>(&self, builder: &mut CircuitBuilder<F, D>) -> Self {
        self.repeated_frobenius(1, builder)
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

    pub fn double_arithmetic_extension(
        &mut self,
        const_0: F,
        const_1: F,
        fixed_multiplicand: ExtensionTarget<D>,
        multiplicand_0: ExtensionTarget<D>,
        addend_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
        addend_1: ExtensionTarget<D>,
    ) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
        let gate = self.add_gate(ArithmeticExtensionGate::new(), vec![const_0, const_1]);

        let wire_fixed_multiplicand = ExtensionTarget::from_range(
            gate,
            ArithmeticExtensionGate::<D>::wires_fixed_multiplicand(),
        );
        let wire_multiplicand_0 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_multiplicand_0());
        let wire_addend_0 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_addend_0());
        let wire_multiplicand_1 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_multiplicand_1());
        let wire_addend_1 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_addend_1());
        let wire_output_0 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_output_0());
        let wire_output_1 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_output_1());

        self.route_extension(fixed_multiplicand, wire_fixed_multiplicand);
        self.route_extension(multiplicand_0, wire_multiplicand_0);
        self.route_extension(addend_0, wire_addend_0);
        self.route_extension(multiplicand_1, wire_multiplicand_1);
        self.route_extension(addend_1, wire_addend_1);
        (wire_output_0, wire_output_1)
    }

    pub fn arithmetic_extension(
        &mut self,
        const_0: F,
        const_1: F,
        multiplicand_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
        addend: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let zero = self.zero_extension();
        self.double_arithmetic_extension(
            const_0,
            const_1,
            multiplicand_0,
            multiplicand_1,
            addend,
            zero,
            zero,
        )
        .0
    }

    pub fn add_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let one = self.one_extension();
        self.arithmetic_extension(F::ONE, F::ONE, one, a, b)
    }

    pub fn add_two_extension(
        &mut self,
        a0: ExtensionTarget<D>,
        b0: ExtensionTarget<D>,
        a1: ExtensionTarget<D>,
        b1: ExtensionTarget<D>,
    ) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
        let one = self.one_extension();
        self.double_arithmetic_extension(F::ONE, F::ONE, one, a0, b0, a1, b1)
    }

    pub fn add_ext_algebra(
        &mut self,
        a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        let mut res = Vec::with_capacity(D);
        let d_even = D & (D ^ 1); // = 2 * (D/2)
        for mut chunk in &(0..d_even).chunks(2) {
            let i = chunk.next().unwrap();
            let j = chunk.next().unwrap();
            let (o0, o1) = self.add_two_extension(a.0[i], b.0[i], a.0[j], b.0[j]);
            res.extend([o0, o1]);
        }
        if D % 2 == 1 {
            res.push(self.add_extension(a.0[D - 1], b.0[D - 1]));
        }
        ExtensionAlgebraTarget(res.try_into().unwrap())
    }

    pub fn add_many_extension(&mut self, terms: &[ExtensionTarget<D>]) -> ExtensionTarget<D> {
        let zero = self.zero_extension();
        let mut terms = terms.to_vec();
        if terms.len() % 2 == 1 {
            terms.push(zero);
        }
        let mut acc0 = zero;
        let mut acc1 = zero;
        for chunk in terms.chunks_exact(2) {
            (acc0, acc1) = self.add_two_extension(acc0, acc1, chunk[0], chunk[1]);
        }
        self.add_extension(acc0, acc1)
    }

    pub fn sub_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let one = self.one_extension();
        self.arithmetic_extension(F::ONE, F::NEG_ONE, one, a, b)
    }

    pub fn sub_two_extension(
        &mut self,
        a0: ExtensionTarget<D>,
        b0: ExtensionTarget<D>,
        a1: ExtensionTarget<D>,
        b1: ExtensionTarget<D>,
    ) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
        let one = self.one_extension();
        self.double_arithmetic_extension(F::ONE, F::NEG_ONE, one, a0, b0, a1, b1)
    }

    pub fn sub_ext_algebra(
        &mut self,
        a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        let mut res = Vec::with_capacity(D);
        let d_even = D & (D ^ 1); // = 2 * (D/2)
        for mut chunk in &(0..d_even).chunks(2) {
            let i = chunk.next().unwrap();
            let j = chunk.next().unwrap();
            let (o0, o1) = self.sub_two_extension(a.0[i], b.0[i], a.0[j], b.0[j]);
            res.extend([o0, o1]);
        }
        if D % 2 == 1 {
            res.push(self.add_extension(a.0[D - 1], b.0[D - 1]));
        }
        ExtensionAlgebraTarget(res.try_into().unwrap())
    }

    pub fn mul_extension_with_const(
        &mut self,
        const_0: F,
        multiplicand_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let zero = self.zero_extension();
        self.double_arithmetic_extension(
            const_0,
            F::ZERO,
            multiplicand_0,
            multiplicand_1,
            zero,
            zero,
            zero,
        )
        .0
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
                res[(i + j) % D] = if i + j < D {
                    self.mul_add_extension(a.0[i], b.0[j], res[(i + j) % D])
                } else {
                    let ai_bi = self.mul_extension(a.0[i], b.0[j]);
                    self.scalar_mul_add_extension(w, ai_bi, res[(i + j) % D])
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
        self.arithmetic_extension(F::ONE, F::ONE, a, b, c)
    }

    /// Like `mul_add`, but for `ExtensionTarget`s.
    pub fn scalar_mul_add_extension(
        &mut self,
        a: Target,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let a_ext = self.convert_to_ext(a);
        self.arithmetic_extension(F::ONE, F::ONE, a_ext, b, c)
    }

    /// Like `mul_sub`, but for `ExtensionTarget`s.
    pub fn scalar_mul_sub_extension(
        &mut self,
        a: Target,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let a_ext = self.convert_to_ext(a);
        self.arithmetic_extension(F::ONE, F::NEG_ONE, a_ext, b, c)
    }

    /// Returns `a * b`, where `b` is in the extension field and `a` is in the base field.
    pub fn scalar_mul_ext(&mut self, a: Target, b: ExtensionTarget<D>) -> ExtensionTarget<D> {
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
