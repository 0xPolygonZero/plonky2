use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::base_sum::BaseSumGate;
use crate::generator::SimpleGenerator;
use crate::target::Target;
use crate::witness::PartialWitness;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Selects `x` or `y` based on `b`, which is assumed to be binary.
    /// In particular, this returns `if b { x } else { y }`.
    /// Note: This does not range-check `b`.
    pub fn select(&mut self, b: Target, x: Target, y: Target) -> Target {
        let b_y_minus_y = self.mul_sub(b, y, y);
        self.mul_sub(b, x, b_y_minus_y)
    }

    /// Left-rotates an array `k` times if `b=1` else return the same array.
    pub fn rotate_fixed(&mut self, b: Target, k: usize, v: Vec<Target>, len: usize) -> Vec<Target> {
        debug_assert_eq!(v.len(), len);
        let mut res = Vec::new();

        for i in 0..len {
            res.push(self.select(b, v[(i + k) % len], v[i]));
        }

        res
    }

    /// Left-rotates an array by `num_rotation`. Assumes that `num_rotation` is range-checked to be
    /// less than `len`.
    /// Note: We assume `len` is less than 8 since we won't use any arity greater than 8 in FRI (maybe?).
    pub fn rotate(&mut self, num_rotation: Target, mut v: Vec<Target>, len: usize) -> Vec<Target> {
        let bits = self.split_le_base::<2>(num_rotation, 3);

        v = self.rotate_fixed(bits[0], 1, v, len);
        v = self.rotate_fixed(bits[1], 2, v, len);
        v = self.rotate_fixed(bits[2], 4, v, len);

        v
    }
}

#[derive(Debug)]
struct UnaryBaseGenerator {
    integer: Target,
    len: usize,
    limbs: Vec<Target>,
}

impl<F: Field> SimpleGenerator<F> for UnaryBaseGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.integer]
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();
        let low = integer_value & ((1 << self.n_log) - 1);
        let high = integer_value >> self.n_log;

        let mut result = PartialWitness::new();
        result.set_target(self.low, F::from_canonical_u64(low));
        result.set_target(self.high, F::from_canonical_u64(high));

        result
    }
}
