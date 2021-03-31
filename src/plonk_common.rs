use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::target::Target;

pub(crate) fn reduce_with_powers<F: Field>(terms: Vec<F>, alpha: F) -> F {
    let mut sum = F::ZERO;
    for &term in terms.iter().rev() {
        sum = sum * alpha + term;
    }
    sum
}

pub(crate) fn reduce_with_powers_recursive<F: Field>(
    builder: &mut CircuitBuilder<F>,
    terms: Vec<Target>,
    alpha: Target,
) -> Target {
    todo!()
}
