use plonky2_field::extension_field::Extendable;

use crate::gates::gate::GateRef;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::witness::PartitionWitness;
use crate::plonk::circuit_data::CircuitConfig;

#[derive(Debug, Clone)]
pub struct Operation<F: RichField + Extendable<D>, const D: usize> {
    pub targets: Vec<Target>,
    /// Generators used to generate the witness.
    // TODO: Do we need only one per operation?
    pub generators: Box<dyn WitnessGenerator<F>>,

    pub gate: GateRef<F, D>,
    pub constants: Vec<F>,
}

// z = builder.add(x,y)
// fn add(x,y) {
//    let z = builder.add_target();
//    let add_generator = todo!();
//    let add_op = Operation { vec![x,y], vec![], vec![z], add_generator, AddGate};
//    z
// }
