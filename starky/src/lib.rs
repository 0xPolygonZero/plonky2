//! A FRI-based STARK implementation over the Goldilocks field, with support
//! for recursive proof verification through the plonky2 SNARK backend.
//!
//! This library is intended to provide all the necessary tools to prove,
//! verify, and recursively verify STARK statements. While the library
//! is tailored for a system with a single STARK, it also is flexible
//! enough to support a multi-STARK system, i.e. a system of independent
//! STARK statements possibly sharing common values. See section below for
//! more information on how to define such a system.
//!
//!
//! # Defining a STARK statement
//!
//! A STARK system is configured by a [`StarkConfig`][crate::config::StarkConfig]
//! defining all the parameters to be used when generating proofs associated
//! to the statement. How constraints should be defined over the STARK trace is
//! defined through the [`Stark`][crate::stark::Stark] trait, that takes a
//! [`StarkEvaluationFrame`][crate::evaluation_frame::StarkEvaluationFrame] of
//! two consecutive rows and a list of public inputs.
//!
//! ### Example: Fibonacci sequence
//!
//! To build a STARK for the modified Fibonacci sequence starting with two
//! user-provided values `x0` and `x1`, one can do the following:
//!
//! ```rust
//! // Imports all basic types.
//! use std::marker::PhantomData;
//! use plonky2::field::extension::{Extendable, FieldExtension};
//! use plonky2::field::packed::PackedField;
//! use plonky2::field::polynomial::PolynomialValues;
//! use plonky2::hash::hash_types::RichField;
//!
//! // Imports to define the constraints of our STARK.
//! use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
//! use starky::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
//! use starky::stark::Stark;
//!
//! // Imports to define the recursive constraints of our STARK.
//! use plonky2::iop::ext_target::ExtensionTarget;
//! use plonky2::plonk::circuit_builder::CircuitBuilder;
//! use starky::util::trace_rows_to_poly_values;
//!
//! // Imports to generate a STARK instance, compute the trace and prove it
//! use plonky2::field::types::Field;
//! use plonky2::plonk::config::GenericConfig;
//! use plonky2::plonk::config::PoseidonGoldilocksConfig;
//! use plonky2::util::timing::TimingTree;
//! use starky::config::StarkConfig;
//! use starky::prover::prove;
//! use starky::verifier::verify_stark_proof;
//!
//!# #[derive(Copy, Clone)]
//! pub struct FibonacciStark<F: RichField + Extendable<D>, const D: usize> {
//!   num_rows: usize,
//!   _phantom: PhantomData<F>,
//! }
//! // Define witness generation.
//! impl<F: RichField + Extendable<D>, const D: usize> FibonacciStark<F, D> {
//!   // The first public input is `x0`.
//!   const PI_INDEX_X0: usize = 0;
//!   // The second public input is `x1`.
//!   const PI_INDEX_X1: usize = 1;
//!   // The third public input is the second element of the last row,
//!   // which should be equal to the `num_rows`-th Fibonacci number.
//!   const PI_INDEX_RES: usize = 2;
//!
//!   pub(crate) fn new(num_rows: usize) -> Self {
//!       Self {
//!           num_rows,
//!           _phantom: PhantomData
//!       }
//!   }
//!
//!   /// Generate the trace using `x0, x1, 0` as initial state values.
//!   fn generate_trace(&self, x0: F, x1: F) -> Vec<PolynomialValues<F>> {
//!       let mut trace_rows = (0..self.num_rows)
//!           .scan([x0, x1, F::ZERO], |acc, _| {
//!               let tmp = *acc;
//!               acc[0] = tmp[1];
//!               acc[1] = tmp[0] + tmp[1];
//!               acc[2] = tmp[2] + F::ONE;
//!               Some(tmp)
//!           })
//!           .collect::<Vec<_>>();
//!       // Transpose the row-wise trace for the prover.
//!       trace_rows_to_poly_values(trace_rows)
//!   }
//! }
//!
//! // Define constraints.
//! const COLUMNS: usize = 3;
//! const PUBLIC_INPUTS: usize = 3;
//!
//! impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for FibonacciStark<F, D> {
//!   type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, P::Scalar, COLUMNS, PUBLIC_INPUTS>
//!   where
//!       FE: FieldExtension<D2, BaseField = F>,
//!       P: PackedField<Scalar = FE>;
//!
//!   type EvaluationFrameTarget =
//!       StarkFrame<ExtensionTarget<D>, ExtensionTarget<D>, COLUMNS, PUBLIC_INPUTS>;
//!
//!   // Define this STARK's constraints.
//!   fn eval_packed_generic<FE, P, const D2: usize>(
//!       &self,
//!       vars: &Self::EvaluationFrame<FE, P, D2>,
//!       yield_constr: &mut ConstraintConsumer<P>,
//!   ) where
//!       FE: FieldExtension<D2, BaseField = F>,
//!       P: PackedField<Scalar = FE>,
//!   {
//!       let local_values = vars.get_local_values();
//!       let next_values = vars.get_next_values();
//!       let public_inputs = vars.get_public_inputs();
//!
//!       // Check public inputs.
//!       yield_constr.constraint_first_row(local_values[0] - public_inputs[Self::PI_INDEX_X0]);
//!       yield_constr.constraint_first_row(local_values[1] - public_inputs[Self::PI_INDEX_X1]);
//!       yield_constr.constraint_last_row(local_values[1] - public_inputs[Self::PI_INDEX_RES]);
//!
//!       // Enforce the Fibonacci transition constraints.
//!       // x0' <- x1
//!       yield_constr.constraint_transition(next_values[0] - local_values[1]);
//!       // x1' <- x0 + x1
//!       yield_constr.constraint_transition(next_values[1] - local_values[0] - local_values[1]);
//!   }
//!
//!   // Define the constraints to recursively verify this STARK.
//!   fn eval_ext_circuit(
//!       &self,
//!       builder: &mut CircuitBuilder<F, D>,
//!       vars: &Self::EvaluationFrameTarget,
//!       yield_constr: &mut RecursiveConstraintConsumer<F, D>,
//!   ) {
//!       let local_values = vars.get_local_values();
//!       let next_values = vars.get_next_values();
//!       let public_inputs = vars.get_public_inputs();
//!
//!       // Check public inputs.
//!       let pis_constraints = [
//!           builder.sub_extension(local_values[0], public_inputs[Self::PI_INDEX_X0]),
//!           builder.sub_extension(local_values[1], public_inputs[Self::PI_INDEX_X1]),
//!           builder.sub_extension(local_values[1], public_inputs[Self::PI_INDEX_RES]),
//!       ];
//!
//!       yield_constr.constraint_first_row(builder, pis_constraints[0]);
//!       yield_constr.constraint_first_row(builder, pis_constraints[1]);
//!       yield_constr.constraint_last_row(builder, pis_constraints[2]);
//!
//!       // Enforce the Fibonacci transition constraints.
//!       // x0' <- x1
//!       let first_col_constraint = builder.sub_extension(next_values[0], local_values[1]);
//!       yield_constr.constraint_transition(builder, first_col_constraint);
//!       // x1' <- x0 + x1
//!       let second_col_constraint = {
//!           let tmp = builder.sub_extension(next_values[1], local_values[0]);
//!           builder.sub_extension(tmp, local_values[1])
//!       };
//!       yield_constr.constraint_transition(builder, second_col_constraint);
//!   }
//!
//!   fn constraint_degree(&self) -> usize {
//!       2
//!   }
//! }
//!
//! // One can then instantiate a new `FibonacciStark` instance, generate an associated
//! // STARK trace, and generate a proof for it.
//!
//! const D: usize = 2;
//! const CONFIG: StarkConfig = StarkConfig::standard_fast_config();
//! type C = PoseidonGoldilocksConfig;
//! type F = <C as GenericConfig<D>>::F;
//! type S = FibonacciStark<F, D>;
//!
//! fn fibonacci<F: Field>(n: usize, x0: F, x1: F) -> F {
//!     (0..n).fold((x0, x1), |acc, _| (acc.1, acc.0 + acc.1)).1
//! }
//!
//! fn fibonacci_stark() {
//!     let num_rows = 1 << 10;
//!     let x0 = F::from_canonical_u32(2);
//!     let x1 = F::from_canonical_u32(7);
//!
//!     let public_inputs = [x0, x1, fibonacci(num_rows - 1, x0, x1)];
//!     let stark = FibonacciStark::<F, D>::new(num_rows);
//!     let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);
//!
//!     let proof = prove::<F, C, S, D>(
//!         stark,
//!         &CONFIG,
//!         trace,
//!         &public_inputs,
//!         None,
//!         &mut TimingTree::default(),
//!     ).expect("We should have a valid proof!");
//!
//!     verify_stark_proof(stark, proof, &CONFIG, None)
//!         .expect("We should be able to verify this proof!")
//! }
//! ```
//!

#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod get_challenges;

pub mod config;
pub mod constraint_consumer;
pub mod cross_table_lookup;
pub mod evaluation_frame;
pub mod lookup;
pub mod proof;
pub mod prover;
pub mod recursive_verifier;
pub mod stark;
pub mod stark_testing;
pub mod util;
mod vanishing_poly;
pub mod verifier;

#[cfg(test)]
pub mod fibonacci_stark;
#[cfg(test)]
pub mod permutation_stark;
#[cfg(test)]
pub mod unconstrained_stark;
