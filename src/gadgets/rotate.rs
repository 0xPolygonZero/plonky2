use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
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
    pub fn select(
        &mut self,
        b: Target,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let b_y_minus_y = self.scalar_mul_sub_extension(b, y, y);
        self.scalar_mul_sub_extension(b, x, b_y_minus_y)
    }

    /// Left-rotates an array `k` times if `b=1` else return the same array.
    pub fn rotate_left_fixed(
        &mut self,
        b: Target,
        k: usize,
        v: &[ExtensionTarget<D>],
        len: usize,
    ) -> Vec<ExtensionTarget<D>> {
        let mut res = Vec::new();

        for i in 0..len {
            res.push(self.select(b, v[(i + k) % len], v[i]));
        }

        res
    }

    /// Left-rotates an array by `num_rotation`. Assumes that `num_rotation` is range-checked to be
    /// less than `len`.
    /// Note: We assume `len` is less than 8 since we won't use any arity greater than 8 in FRI (maybe?).
    pub fn rotate_left_from_bits(
        &mut self,
        num_rotation_bits: &[Target],
        v: &[ExtensionTarget<D>],
        len_log: usize,
    ) -> Vec<ExtensionTarget<D>> {
        debug_assert_eq!(num_rotation_bits.len(), len_log);
        let len = 1 << len_log;
        debug_assert_eq!(v.len(), len);
        let mut v = v.to_vec();

        for i in 0..len_log {
            v = self.rotate_left_fixed(num_rotation_bits[i], 1 << i, &v, len);
        }

        v
    }

    /// Left-rotates an array by `num_rotation`. Assumes that `num_rotation` is range-checked to be
    /// less than `len`.
    /// Note: We assume `len` is a power of two less than or equal to 8, since we won't use any
    /// arity greater than 8 in FRI (maybe?).
    pub fn rotate_left(
        &mut self,
        num_rotation: Target,
        v: &[ExtensionTarget<D>],
        len_log: usize,
    ) -> Vec<ExtensionTarget<D>> {
        let len = 1 << len_log;
        debug_assert_eq!(v.len(), len);
        let bits = self.split_le(num_rotation, len_log);

        self.rotate_left_from_bits(&bits, v, len_log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;

    fn real_rotate<const D: usize>(
        num_rotation: usize,
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let mut res = v.to_vec();
        res.rotate_left(num_rotation);
        res
    }

    fn test_rotate_given_len(len_log: usize) {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let len = 1 << len_log;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let v = (0..len)
            .map(|_| builder.constant_extension(FF::rand()))
            .collect::<Vec<_>>();

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let rotated = real_rotate(i, &v);
            let purported_rotated = builder.rotate_left(it, &v, len_log);

            for (x, y) in rotated.into_iter().zip(purported_rotated) {
                builder.assert_equal_extension(x, y);
            }
        }

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());
    }

    #[test]
    fn test_rotate() {
        for len_log in 1..3 {
            test_rotate_given_len(len_log);
        }
    }
}
