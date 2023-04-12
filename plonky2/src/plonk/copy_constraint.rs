use alloc::string::String;

use crate::iop::target::Target;

/// A named copy constraint.
pub struct CopyConstraint {
    pub pair: (Target, Target),
    pub name: String,
}

impl From<(Target, Target)> for CopyConstraint {
    fn from(pair: (Target, Target)) -> Self {
        Self {
            pair,
            name: String::new(),
        }
    }
}

impl CopyConstraint {
    pub fn new(pair: (Target, Target), name: String) -> Self {
        Self { pair, name }
    }
}
