use plonky2_field::extension_field::Extendable;

use crate::gates::gate::GateRef;
use crate::hash::hash_types::RichField;
use crate::iop::generator::WitnessGenerator;
use crate::iop::target::Target;

pub struct Operation<F: RichField + Extendable<D>, const D: usize> {
    inputs: Vec<Target>,
    outputs: Vec<Target>,
    /// Generators used to generate the witness.
    // TODO: Do we need only one per operation?
    generators: Vec<Box<dyn WitnessGenerator<F>>>,

    gate: GateRef<F, D>,
}

// z = builder.add(x,y)
// fn add(x,y) {
//    let z = builder.add_target();
//    let add_generator = todo!();
//    let add_op = Operation { vec![x,y], vec![z], add_generator, AddGate};
//    z
// }
