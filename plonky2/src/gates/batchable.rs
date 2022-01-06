use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use plonky2_field::extension_field::Extendable;

use crate::gates::gate::Gate;
use crate::hash::hash_types::RichField;
use crate::plonk::circuit_builder::CircuitBuilder;

pub trait BatchableGate<F: RichField + Extendable<D>, const D: usize>: Gate<F, D> {
    type Parameters: Copy;

    fn find_available_slot(&self, params: Self::Parameters) -> (usize, usize);

    fn fill_gate(&self, params: Self::Parameters, builder: &mut CircuitBuilder<F, D>);
}

pub struct CurrentSlot<F: RichField + Extendable<D>, const D: usize, G: BatchableGate<F, D>> {
    current_slot: HashMap<G::Parameters, (usize, usize)>,
}

// pub struct Yo<F: RichField + Extendable<D>, const D: usize>(
//     CurrentSlot<F, D, dyn BatchableGate<F, D>>,
// );
pub struct GateRef<F: RichField + Extendable<D>, const D: usize>(
    pub(crate) Arc<dyn BatchableGate<F, D>>,
);
