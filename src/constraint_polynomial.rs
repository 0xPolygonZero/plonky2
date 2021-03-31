use crate::field::field::Field;
use crate::target::Target;

#[derive(Copy, Clone)]
pub struct EvaluationVars<'a, F: Field> {
    pub(crate) local_constants: &'a [F],
    pub(crate) next_constants: &'a [F],
    pub(crate) local_wires: &'a [F],
    pub(crate) next_wires: &'a [F],
}

pub struct EvaluationTargets<'a> {
    pub(crate) local_constants: &'a [Target],
    pub(crate) next_constants: &'a [Target],
    pub(crate) local_wires: &'a [Target],
    pub(crate) next_wires: &'a [Target],
}
