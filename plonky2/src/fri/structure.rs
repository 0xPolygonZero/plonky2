//! Information about the structure of a FRI instance, in terms of the oracles and polynomials
//! involved, and the points they are opened at.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::ops::Range;

use crate::field::extension::Extendable;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;

/// Describes an instance of a FRI-based batch opening.
#[derive(Clone, Debug)]
pub struct FriInstanceInfo<F: RichField + Extendable<D>, const D: usize> {
    /// The oracles involved, not counting oracles created during the commit phase.
    pub oracles: Vec<FriOracleInfo>,
    /// Batches of openings, where each batch is associated with a particular point.
    pub batches: Vec<FriBatchInfo<F, D>>,
}

/// Describes an instance of a FRI-based batch opening.
#[derive(Debug)]
pub struct FriInstanceInfoTarget<const D: usize> {
    /// The oracles involved, not counting oracles created during the commit phase.
    pub oracles: Vec<FriOracleInfo>,
    /// Batches of openings, where each batch is associated with a particular point.
    pub batches: Vec<FriBatchInfoTarget<D>>,
}

#[derive(Copy, Clone, Debug)]
pub struct FriOracleInfo {
    pub num_polys: usize,
    pub blinding: bool,
}

/// A batch of openings at a particular point.
#[derive(Clone, Debug)]
pub struct FriBatchInfo<F: RichField + Extendable<D>, const D: usize> {
    pub point: F::Extension,
    pub polynomials: Vec<FriPolynomialInfo>,
}

/// A batch of openings at a particular point.
#[derive(Debug)]
pub struct FriBatchInfoTarget<const D: usize> {
    pub point: ExtensionTarget<D>,
    pub polynomials: Vec<FriPolynomialInfo>,
}

#[derive(Copy, Clone, Debug)]
pub struct FriPolynomialInfo {
    /// Index into `FriInstanceInfo`'s `oracles` list.
    pub oracle_index: usize,
    /// Index of the polynomial within the oracle.
    pub polynomial_index: usize,
}

impl FriPolynomialInfo {
    pub fn from_range(
        oracle_index: usize,
        polynomial_indices: Range<usize>,
    ) -> Vec<FriPolynomialInfo> {
        polynomial_indices
            .map(|polynomial_index| FriPolynomialInfo {
                oracle_index,
                polynomial_index,
            })
            .collect()
    }
}

/// Opened values of each polynomial.
#[derive(Debug)]
pub struct FriOpenings<F: RichField + Extendable<D>, const D: usize> {
    pub batches: Vec<FriOpeningBatch<F, D>>,
}

/// Opened values of each polynomial that's opened at a particular point.
#[derive(Debug)]
pub struct FriOpeningBatch<F: RichField + Extendable<D>, const D: usize> {
    pub values: Vec<F::Extension>,
}

/// Opened values of each polynomial.
#[derive(Debug)]
pub struct FriOpeningsTarget<const D: usize> {
    pub batches: Vec<FriOpeningBatchTarget<D>>,
}

/// Opened values of each polynomial that's opened at a particular point.
#[derive(Debug)]
pub struct FriOpeningBatchTarget<const D: usize> {
    pub values: Vec<ExtensionTarget<D>>,
}
