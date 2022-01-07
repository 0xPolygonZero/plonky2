use plonky2_field::extension_field::Extendable;

use crate::gates::gate::GateRef;
use crate::hash::hash_types::RichField;
use crate::iop::generator::WitnessGenerator;

pub struct Operation<F: RichField + Extendable<D>, const D: usize> {
    /// Generators used to generate the witness.
    // TODO: Do we need only one per operation?
    generators: Vec<Box<dyn WitnessGenerator<F>>>,

    gate: GateRef<F, D>,
}
