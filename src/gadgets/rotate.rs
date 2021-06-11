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
    pub fn rotate_fixed(&mut self, b: Target, k: usize, v: &[Target], len: usize) -> Vec<Target> {
        let mut res = Vec::new();

        for i in 0..len {
            res.push(self.select(b, v[(i + k) % len], v[i]));
        }

        res
    }

    /// Left-rotates an array by `num_rotation`. Assumes that `num_rotation` is range-checked to be
    /// less than `len`.
    /// Note: We assume `len` is less than 8 since we won't use any arity greater than 8 in FRI (maybe?).
    pub fn rotate(&mut self, num_rotation: Target, v: &[Target], len: usize) -> Vec<Target> {
        debug_assert_eq!(v.len(), len);
        let bits = self.split_le_base::<2>(num_rotation, 3);

        let v = self.rotate_fixed(bits[0], 1, v, len);
        let v = self.rotate_fixed(bits[1], 2, &v, len);
        let v = self.rotate_fixed(bits[2], 4, &v, len);

        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;

    fn real_rotate(num_rotation: usize, v: &[Target]) -> Vec<Target> {
        let mut res = v.to_vec();
        res.rotate_left(num_rotation);
        res
    }

    fn test_rotate_given_len(len: usize) {
        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let v = (0..len)
            .map(|_| builder.constant(F::rand()))
            .collect::<Vec<_>>(); // 416 = 1532 in base 6.

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let rotated = real_rotate(i, &v);
            let purported_rotated = builder.rotate(it, &v, len);

            for (x, y) in rotated.into_iter().zip(purported_rotated) {
                builder.assert_equal(x, y);
            }
        }

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());
    }

    #[test]
    fn test_rotate() {
        for i_log in 1..4 {
            test_rotate_given_len(1 << i_log);
        }
    }
}
