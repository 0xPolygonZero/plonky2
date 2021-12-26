#![allow(incomplete_features)]
#![allow(const_evaluatable_unchecked)]
#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::return_self_not_must_use)]
#![feature(asm_sym)]
#![feature(generic_const_exprs)]
#![feature(specialization)]
#![feature(stdsimd)]

pub mod curve;
pub mod fri;
pub mod gadgets;
pub mod gates;
pub mod hash;
pub mod iop;
pub mod plonk;
pub mod util;

// Set up Jemalloc
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
