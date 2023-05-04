use crate::field::extension::Extendable;
use crate::hash::hash_types::RichField;
use crate::hash::hashing::PlonkyPermutation;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::AlgebraicHasher;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn permute<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: [Target; H::Permutation::WIDTH],
    ) -> [Target; H::Permutation::WIDTH] {
        // We don't want to swap any inputs, so set that wire to 0.
        let _false = self._false();
        self.permute_swapped::<H>(inputs, _false)
    }

    /// Conditionally swap two chunks of the inputs (useful in verifying Merkle proofs), then apply
    /// a cryptographic permutation.
    pub(crate) fn permute_swapped<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: [Target; H::Permutation::WIDTH],
        swap: BoolTarget,
    ) -> [Target; H::Permutation::WIDTH] {
        H::permute_swapped(inputs, swap, self)
    }
}
