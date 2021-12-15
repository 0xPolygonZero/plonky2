use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::gmimc::GMiMCGate;
use crate::gates::poseidon::PoseidonGate;
use crate::hash::gmimc::GMiMC;
use crate::hash::hashing::SPONGE_WIDTH;
use crate::hash::poseidon::Poseidon;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::AlgebraicHasher;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn permute<const W: usize>(&mut self, inputs: [Target; W]) -> [Target; W]
    where
        F: GMiMC<W> + Poseidon<W>,
        [(); W - 1]:,
    {
    pub fn permute<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: [Target; SPONGE_WIDTH],
    ) -> [Target; SPONGE_WIDTH] {
        // We don't want to swap any inputs, so set that wire to 0.
        let _false = self._false();
        self.permute_swapped::<H>(inputs, _false)
    }

    /// Conditionally swap two chunks of the inputs (useful in verifying Merkle proofs), then apply
    /// a cryptographic permutation.
    pub(crate) fn permute_swapped<const W: usize>(
        &mut self,
        inputs: [Target; W],
        swap: BoolTarget,
    ) -> [Target; W]
    where
        F: GMiMC<W> + Poseidon<W>,
        [(); W - 1]:,
    {
        match HASH_FAMILY {
            HashFamily::GMiMC => self.gmimc_permute_swapped(inputs, swap),
            HashFamily::Poseidon => self.poseidon_permute_swapped(inputs, swap),
        }
    }

    /// Conditionally swap two chunks of the inputs (useful in verifying Merkle proofs), then apply
    /// the GMiMC permutation.
    pub(crate) fn gmimc_permute_swapped<const W: usize>(
    pub(crate) fn permute_swapped<H: AlgebraicHasher<F>>(
        &mut self,
        inputs: [Target; SPONGE_WIDTH],
        swap: BoolTarget,
    ) -> [Target; SPONGE_WIDTH] {
        H::permute_swapped(inputs, swap, self)
    }
}
