use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::target::Target;
use crate::util::log2_ceil;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Selects `x` or `y` based on `b`, which is assumed to be binary.
    /// In particular, this returns `if b { x } else { y }`.
    /// Note: This does not range-check `b`.
    // TODO: This uses 10 gates per call. If addends are added to `MulExtensionGate`, this will be
    // reduced to 2 gates. We could also use a new degree 2 `SelectGate` for this.
    // If `num_routed_wire` is larger than 26, we could batch two `select` in one gate.
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
    ) -> Vec<ExtensionTarget<D>> {
        let len = v.len();
        debug_assert!(k < len, "Trying to rotate by more than the vector length.");
        let mut res = Vec::new();

        for i in 0..len {
            res.push(self.select(b, v[(i + k) % len], v[i]));
        }

        res
    }

    /// Left-rotates an array `k` times if `b=1` else return the same array.
    pub fn rotate_right_fixed(
        &mut self,
        b: Target,
        k: usize,
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let len = v.len();
        debug_assert!(k < len, "Trying to rotate by more than the vector length.");
        let mut res = Vec::new();

        for i in 0..len {
            res.push(self.select(b, v[(len + i - k) % len], v[i]));
        }

        res
    }

    /// Left-rotates an vector by the `Target` having bits given in little-endian by `num_rotation_bits`.
    pub fn rotate_left_from_bits(
        &mut self,
        num_rotation_bits: &[Target],
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let mut v = v.to_vec();

        for i in 0..num_rotation_bits.len() {
            v = self.rotate_left_fixed(num_rotation_bits[i], 1 << i, &v);
        }

        v
    }

    pub fn rotate_right_from_bits(
        &mut self,
        num_rotation_bits: &[Target],
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let mut v = v.to_vec();

        for i in 0..num_rotation_bits.len() {
            v = self.rotate_right_fixed(num_rotation_bits[i], 1 << i, &v);
        }

        v
    }

    /// Left-rotates an array by `num_rotation`. Assumes that `num_rotation` is range-checked to be
    /// less than `2^len_bits`.
    pub fn rotate_left(
        &mut self,
        num_rotation: Target,
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let len_bits = log2_ceil(v.len());
        let bits = self.split_le(num_rotation, len_bits);

        self.rotate_left_from_bits(&bits, v)
    }

    pub fn rotate_right(
        &mut self,
        num_rotation: Target,
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let len_bits = log2_ceil(v.len());
        let bits = self.split_le(num_rotation, len_bits);

        self.rotate_right_from_bits(&bits, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field::Field;
    use crate::verifier::verify;
    use crate::witness::PartialWitness;

    fn real_rotate<const D: usize>(
        num_rotation: usize,
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let mut res = v.to_vec();
        res.rotate_left(num_rotation);
        res
    }

    fn test_rotate_given_len(len: usize) {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let v = (0..len)
            .map(|_| builder.constant_extension(FF::rand()))
            .collect::<Vec<_>>();

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let rotated = real_rotate(i, &v);
            let purported_rotated = builder.rotate_left(it, &v);

            for (x, y) in rotated.into_iter().zip(purported_rotated) {
                builder.assert_equal_extension(x, y);
            }
        }

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());

        verify(proof, &data.verifier_only, &data.common).unwrap();
    }

    #[test]
    fn test_rotate() {
        for len in 1..5 {
            test_rotate_given_len(len);
        }
    }
}
