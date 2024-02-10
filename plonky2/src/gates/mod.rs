//! plonky2 custom gates.
//!
//! Vanilla Plonk arithmetization only supports basic fan-in 2 / fan-out 1 arithmetic gates,
//! each of the form
//!
//! $$ a.b.q_M + a.q_L + b.q_R + c.q_O + q_C = 0 $$
//!
//! where:
//! - $q_M$, $q_L$, $q_R$ and $q_O$ are boolean selectors,
//! - $a$, $b$ and $c$ are values used as inputs and output respectively,
//! - $q_C$ is a constant (possibly 0).
//!
//! This allows expressing simple operations like multiplication, addition, etc. For
//! instance, to define a multiplication, one can set $q_M=1$, $q_L=q_R=0$, $q_O = -1$ and $q_C = 0$.
//!
//! Hence, the gate equation simplifies to $a.b - c = 0$, or equivalently to $a.b = c$.
//!
//! However, such a gate is fairly limited for more complex computations. Hence, when a computation may
//! require too many of these "vanilla" gates, or when a computation arises often within the same circuit,
//! one may want to construct a tailored custom gate. These custom gates can use more selectors and are
//! not necessarily limited to 2 inputs + 1 output = 3 wires.
//! For instance, plonky2 supports natively a custom Poseidon hash gate that uses 135 wires.

// Gates have `new` methods that return `GateRef`s.

pub mod arithmetic_base;
pub mod arithmetic_extension;
pub mod base_sum;
pub mod constant;
pub mod coset_interpolation;
pub mod exponentiation;
pub mod gate;
pub mod lookup;
pub mod lookup_table;
pub mod multiplication_extension;
pub mod noop;
pub mod packed_util;
pub mod poseidon;
pub mod poseidon_mds;
pub mod public_input;
pub mod random_access;
pub mod reducing;
pub mod reducing_extension;
pub(crate) mod selectors;
pub mod util;

// Can't use #[cfg(test)] here because it needs to be visible to other crates.
// See https://github.com/rust-lang/cargo/issues/8379
#[cfg(any(feature = "gate_testing", test))]
pub mod gate_testing;
