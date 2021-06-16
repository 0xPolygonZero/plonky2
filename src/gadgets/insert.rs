use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::target::Target;
use crate::util::bits_u64;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Inserts a `Target` in a vector at a non-deterministic index. This is done by rotating to the
    /// left, inserting at 0 and then rotating to the right.
    /// Note: `index` is not range-checked.
    pub fn insert(
        &mut self,
        index: Target,
        element: ExtensionTarget<D>,
        mut v: Vec<ExtensionTarget<D>>,
    ) -> Vec<ExtensionTarget<D>> {
        let len = v.len();
        let len_bits = bits_u64(len as u64);

        let mut v = self.rotate_left(index, &v, len_bits);

        v.insert(0, element);

        let len_bits = bits_u64(len as u64 + 1);
        self.rotate_right(index, &v, len_bits)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field::Field;
    use crate::witness::PartialWitness;

    fn real_insert<const D: usize>(
        index: usize,
        element: ExtensionTarget<D>,
        v: &[ExtensionTarget<D>],
    ) -> Vec<ExtensionTarget<D>> {
        let mut res = v.to_vec();
        res.insert(index, element);
        res
    }

    fn test_insert_given_len(len_log: usize) {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let len = 1 << len_log;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let v = (0..len - 1)
            .map(|_| builder.constant_extension(FF::rand()))
            .collect::<Vec<_>>();

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let elem = builder.constant_extension(FF::rand());
            let inserted = real_insert(i, elem, &v);
            let purported_inserted = builder.insert(it, elem, v.clone());

            for (x, y) in inserted.into_iter().zip(purported_inserted) {
                builder.route_extension(x, y);
            }
        }

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());
    }

    #[test]
    fn test_insert() {
        for len_log in 1..3 {
            test_insert_given_len(len_log);
        }
    }
}
