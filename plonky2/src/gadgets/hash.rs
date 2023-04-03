use crate::field::extension::Extendable;
use crate::hash::hash_types::RichField;
use crate::hash::hashing::HashConfig;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::AlgebraicHasher;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn permute<HC: HashConfig, H: AlgebraicHasher<F, HC>>(
        &mut self,
        inputs: [Target; HC::WIDTH],
    ) -> [Target; HC::WIDTH] {
        // We don't want to swap any inputs, so set that wire to 0.
        let _false = self._false();
        self.permute_swapped::<HC, H>(inputs, _false)
    }

    /// Conditionally swap two chunks of the inputs (useful in verifying Merkle proofs), then apply
    /// a cryptographic permutation.
    pub(crate) fn permute_swapped<HC: HashConfig, H: AlgebraicHasher<F, HC>>(
        &mut self,
        inputs: [Target; HC::WIDTH],
        swap: BoolTarget,
    ) -> [Target; HC::WIDTH] {
        H::permute_swapped(inputs, swap, self)
    }
}
