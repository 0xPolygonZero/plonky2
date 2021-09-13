mod common;
mod crandall;
mod goldilocks;
mod packed_prime_field;

use crate::field::crandall_field::CrandallField;
use crate::field::goldilocks_field::GoldilocksField;

use packed_prime_field::PackedPrimeField;

pub type PackedCrandallAVX2 = PackedPrimeField<CrandallField>;
pub type PackedGoldilocksAVX2 = PackedPrimeField<GoldilocksField>;
