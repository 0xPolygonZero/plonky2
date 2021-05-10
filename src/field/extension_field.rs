use crate::field::crandall_field::CrandallField;
use crate::field::field::Field;
use std::fmt::{Debug, Display, Formatter};

pub trait QuarticFieldExtension: Field {
    type BaseField: Field;

    // Element W of BaseField, such that `X^4 - W` is irreducible over BaseField.
    const W: Self::BaseField;

    fn to_canonical_representation(&self) -> [Self::BaseField; 4];
}

pub struct QuarticCrandallField([CrandallField; 4]);

impl QuarticFieldExtension for QuarticCrandallField {
    type BaseField = CrandallField;
    // Verifiable in Sage with
    // ``R.<x> = GF(p)[]; assert (x^4 -3).is_irreducible()`.
    const W: Self::BaseField = CrandallField::from_canonical_u64(3);

    fn to_canonical_representation(&self) -> [Self::BaseField; 4] {
        self.0
    }
}

impl Field for QuarticCrandallField {
    const ZERO: Self = Self([CrandallField::ZERO; 4]);
    const ONE: Self = Self([
        CrandallField::ONE,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);
    const TWO: Self = Self([
        CrandallField::TWO,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);
    const NEG_ONE: Self = Self([
        CrandallField::NEG_ONE,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);

    // Does not fit in 64-bits.
    const ORDER: u64 = 0;
    const TWO_ADICITY: usize = 30;
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self([
        CrandallField::from_canonical_u64(3),
        CrandallField::ONE,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);
    const POWER_OF_TWO_GENERATOR: Self = Self([
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::from_canonical_u64(14096607364803438105),
    ]);

    fn try_inverse(&self) -> Option<Self> {
        todo!()
    }

    fn to_canonical_u64(&self) -> u64 {
        todo!()
    }

    fn from_canonical_u64(n: u64) -> Self {
        todo!()
    }
}

impl Display for QuarticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} + {}*a + {}*a^2 + {}*a^3",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl Debug for QuarticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
