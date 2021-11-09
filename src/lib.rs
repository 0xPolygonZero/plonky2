#![feature(asm)]
#![feature(destructuring_assignment)]
#![feature(generic_const_exprs)]
#![feature(specialization)]
#![feature(stdsimd)]

pub mod field;
pub mod fri;
pub mod gadgets;
pub mod gates;
pub mod hash;
pub mod iop;
pub mod plonk;
pub mod polynomial;
pub mod util;

// Set up Jemalloc
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
