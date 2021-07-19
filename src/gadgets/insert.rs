use std::marker::PhantomData;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gates::insertion::InsertionGate;
use crate::target::Target;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Inserts a `Target` in a vector at a non-deterministic index.
    /// Note: `index` is not range-checked.
    pub fn insert(
        &mut self,
        index: Target,
        element: ExtensionTarget<D>,
        v: Vec<ExtensionTarget<D>>,
    ) -> Vec<ExtensionTarget<D>> {
        let gate = InsertionGate::<F, D> {
            vec_size: v.len(),
            _phantom: PhantomData,
        };
        let gate_index = self.add_gate_no_constants(InsertionGate::new(v.len()));

        v.iter().enumerate().for_each(|(i, &val)| {
            self.route_extension(
                val,
                ExtensionTarget::from_range(gate_index, gate.wires_original_list_item(i)),
            );
        });
        self.route(
            index,
            Target::wire(gate_index, gate.wires_insertion_index()),
        );
        self.route_extension(
            element,
            ExtensionTarget::from_range(gate_index, gate.wires_element_to_insert()),
        );

        (0..=v.len())
            .map(|i| ExtensionTarget::from_range(gate_index, gate.wires_output_list_item(i)))
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field::Field;
    use crate::verifier::verify;
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

    fn test_insert_given_len(len_log: usize) -> Result<()> {
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

            assert_eq!(inserted.len(), purported_inserted.len());

            for (x, y) in inserted.into_iter().zip(purported_inserted) {
                builder.route_extension(x, y);
            }
        }

        let data = builder.build();
        let proof = data.prove(PartialWitness::new())?;

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_insert() -> Result<()> {
        for len_log in 1..3 {
            test_insert_given_len(len_log)?;
        }
        Ok(())
    }
}
