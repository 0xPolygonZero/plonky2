use std::{fmt, ptr};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::rc::Rc;

use num::{BigUint, FromPrimitive, One, Zero};

use crate::field::field::Field;
use crate::target::Target;
use crate::wire::Wire;

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
