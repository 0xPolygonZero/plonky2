use std::hash::{Hash, Hasher};
use std::sync::Arc;

use plonky2_field::extension_field::Extendable;

use crate::gates::gate::GateRef;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::witness::PartitionWitness;
use crate::plonk::circuit_data::CircuitConfig;

pub trait Operation<F: RichField + Extendable<D>, const D: usize>:
    SimpleGenerator<F> + Send + Sync
{
    fn id(&self) -> String;
    fn targets(&self) -> Vec<Target>;
    fn gate(&self) -> Option<GateRef<F, D>>;
    fn constants(&self) -> Vec<F>;
}

pub struct OperationRef<F: RichField + Extendable<D>, const D: usize>(pub Arc<dyn Operation<F, D>>);

impl<F: RichField + Extendable<D>, const D: usize> PartialEq for OperationRef<F, D> {
    fn eq(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Hash for OperationRef<F, D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id().hash(state)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Eq for OperationRef<F, D> {}

// #[derive(Debug, Clone)]
// pub struct Operation<F: RichField + Extendable<D>, const D: usize> {
//     pub targets: Vec<Target>,
//     /// Generators used to generate the witness.
//     // TODO: Do we need only one per operation?
//     pub generators: Box<dyn WitnessGenerator<F>>,
//
//     pub gate: GateRef<F, D>,
//     pub constants: Vec<F>,
// }

// z = builder.add(x,y)
// fn add(x,y) {
//    let z = builder.add_target();
//    let add_generator = todo!();
//    let add_op = Operation { vec![x,y], vec![], vec![z], add_generator, AddGate};
//    z
// }
