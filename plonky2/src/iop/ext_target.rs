#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::ops::Range;

use crate::field::extension::algebra::ExtensionAlgebra;
use crate::field::extension::{Extendable, FieldExtension, OEF};
use crate::field::types::Field;
use crate::hash::hash_types::RichField;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

/// `Target`s representing an element of an extension field.
///
/// This is typically used in recursion settings, where the outer circuit must verify
/// a proof satisfying an inner circuit's statement, which is verified using arithmetic
/// in an extension of the base field.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ExtensionTarget<const D: usize>(pub [Target; D]);

impl<const D: usize> Default for ExtensionTarget<D> {
    fn default() -> Self {
        Self([Target::default(); D])
    }
}

impl<const D: usize> ExtensionTarget<D> {
    pub const fn to_target_array(&self) -> [Target; D] {
        self.0
    }

    pub fn frobenius<F: RichField + Extendable<D>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        self.repeated_frobenius(1, builder)
    }

    pub fn repeated_frobenius<F: RichField + Extendable<D>>(
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
        let k = (F::order() - 1u32) / (D as u64);
        let z0 = F::Extension::W.exp_biguint(&(k * count as u64));
        #[allow(clippy::needless_collect)]
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

    pub fn from_range(row: usize, range: Range<usize>) -> Self {
        debug_assert_eq!(range.end - range.start, D);
        Target::wires_from_range(row, range).try_into().unwrap()
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
    pub const fn to_ext_target_array(&self) -> [ExtensionTarget<D>; D] {
        self.0
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
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

    pub fn neg_one_extension(&mut self) -> ExtensionTarget<D> {
        self.constant_extension(F::Extension::NEG_ONE)
    }

    pub fn zero_ext_algebra(&mut self) -> ExtensionAlgebraTarget<D> {
        self.constant_ext_algebra(ExtensionAlgebra::ZERO)
    }

    pub fn convert_to_ext(&mut self, t: Target) -> ExtensionTarget<D> {
        let zero = self.zero();
        t.to_ext_target(zero)
    }

    pub fn convert_to_ext_algebra(&mut self, et: ExtensionTarget<D>) -> ExtensionAlgebraTarget<D> {
        let zero = self.zero_extension();
        let mut arr = [zero; D];
        arr[0] = et;
        ExtensionAlgebraTarget(arr)
    }
}

/// Flatten the slice by sending every extension target to its D-sized canonical representation.
pub fn flatten_target<const D: usize>(l: &[ExtensionTarget<D>]) -> Vec<Target> {
    l.iter()
        .flat_map(|x| x.to_target_array().to_vec())
        .collect()
}

/// Batch every D-sized chunks into extension targets.
pub fn unflatten_target<const D: usize>(l: &[Target]) -> Vec<ExtensionTarget<D>> {
    debug_assert_eq!(l.len() % D, 0);
    l.chunks_exact(D)
        .map(|c| c.to_vec().try_into().unwrap())
        .collect()
}
