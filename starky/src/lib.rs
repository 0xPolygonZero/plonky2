// TODO: Remove these when crate is closer to being finished.
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]
#![allow(clippy::diverging_sub_expression)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

pub mod config;
pub mod constraint_consumer;
pub mod proof;
pub mod prover;
pub mod stark;
pub mod vars;
