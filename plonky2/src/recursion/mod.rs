//! Recursion logic for verifying recursively plonky2 circuits.
//!
//! This module also provides ways to perform conditional recursive verification
//! (between two different circuits, depending on a condition), and cyclic
//! recursion where a circuit implements its own verification logic.

pub mod conditional_recursive_verifier;
pub mod cyclic_recursion;
pub mod dummy_circuit;
pub mod recursive_verifier;
