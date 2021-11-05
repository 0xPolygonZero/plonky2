use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::random_access::RandomAccessGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Finds the last available random access gate with the given `vec_size` or add one if there aren't any.
    /// Returns `(g,i)` such that there is a random access gate with the given `vec_size` at index
    /// `g` and the gate's `i`-th random access is available.
    fn find_random_access_gate(&mut self, vec_size: usize) -> (usize, usize) {
        let (gate, i) = self
            .free_random_access
            .get(&vec_size)
            .copied()
            .unwrap_or_else(|| {
                let gate = self.add_gate(
                    RandomAccessGate::new_from_config(&self.config, vec_size),
                    vec![],
                );
                (gate, 0)
            });

        // Update `free_random_access` with new values.
        if i < RandomAccessGate::<F, D>::max_num_copies(
            self.config.num_routed_wires,
            self.config.num_wires,
            vec_size,
        ) - 1
        {
            self.free_random_access.insert(vec_size, (gate, i + 1));
        } else {
            self.free_random_access.remove(&vec_size);
        }

        (gate, i)
    }

    /// Checks that a `Target` matches a vector at a non-deterministic index.
    /// Note: `access_index` is not range-checked.
    pub fn random_access(&mut self, access_index: Target, claimed_element: Target, v: Vec<Target>) {
        let vec_size = v.len();
        debug_assert!(vec_size > 0);
        if vec_size == 1 {
            return self.connect(claimed_element, v[0]);
        }
        let (gate_index, copy) = self.find_random_access_gate(vec_size);
        let dummy_gate = RandomAccessGate::<F, D>::new_from_config(&self.config, vec_size);

        v.iter().enumerate().for_each(|(i, &val)| {
            self.connect(
                val,
                Target::wire(gate_index, dummy_gate.wire_list_item(i, copy)),
            );
        });
        self.connect(
            access_index,
            Target::wire(gate_index, dummy_gate.wire_access_index(copy)),
        );
        self.connect(
            claimed_element,
            Target::wire(gate_index, dummy_gate.wire_claimed_element(copy)),
        );
    }

    /// Checks that an `ExtensionTarget` matches a vector at a non-deterministic index.
    /// Note: `access_index` is not range-checked.
    pub fn random_access_extension(
        &mut self,
        access_index: Target,
        claimed_element: ExtensionTarget<D>,
        v: Vec<ExtensionTarget<D>>,
    ) {
        for i in 0..D {
            self.random_access(
                access_index,
                claimed_element.0[i],
                v.iter().map(|et| et.0[i]).collect(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    fn test_random_access_given_len(len_log: usize) -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        let len = 1 << len_log;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let vec = FF::rand_vec(len);
        let v: Vec<_> = vec.iter().map(|x| builder.constant_extension(*x)).collect();

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let elem = builder.constant_extension(vec[i]);
            builder.random_access_extension(it, elem, v.clone());
        }

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_random_access() -> Result<()> {
        for len_log in 1..3 {
            test_random_access_given_len(len_log)?;
        }
        Ok(())
    }
}
