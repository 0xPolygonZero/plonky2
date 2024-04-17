#[cfg(not(feature = "std"))]
use alloc::string::String;

use crate::iop::target::Target;

/// A named copy constraint.
#[derive(Debug)]
pub struct CopyConstraint {
    pub pair: (Target, Target),
    #[allow(dead_code)]
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
    pub const fn new(pair: (Target, Target), name: String) -> Self {
        Self { pair, name }
    }
}
