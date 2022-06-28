use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::insertion_gate::InsertionGate;

pub trait CircuitBuilderInsert<F: RichField + Extendable<D>, const D: usize> {
    /// Inserts a `Target` in a vector at a non-deterministic index.
    /// Note: `index` is not range-checked.
    fn insert(
        &mut self,
        index: Target,
        element: ExtensionTarget<D>,
        v: Vec<ExtensionTarget<D>>,
    ) -> Vec<ExtensionTarget<D>>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderInsert<F, D>
    for CircuitBuilder<F, D>
{
    fn insert(
        &mut self,
        index: Target,
        element: ExtensionTarget<D>,
        v: Vec<ExtensionTarget<D>>,
    ) -> Vec<ExtensionTarget<D>> {
        let gate = InsertionGate::new(v.len());
        let row = self.add_gate(gate.clone(), vec![]);

        v.iter().enumerate().for_each(|(i, &val)| {
            self.connect_extension(
                val,
                ExtensionTarget::from_range(row, gate.wires_original_list_item(i)),
            );
        });
        self.connect(index, Target::wire(row, gate.wires_insertion_index()));
        self.connect_extension(
            element,
            ExtensionTarget::from_range(row, gate.wires_element_to_insert()),
        );

        (0..=v.len())
            .map(|i| ExtensionTarget::from_range(row, gate.wires_output_list_item(i)))
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::types::Field;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use super::*;

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
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        let len = 1 << len_log;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);
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
                builder.connect_extension(x, y);
            }
        }

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;

        data.verify(proof)
    }

    #[test]
    fn test_insert() -> Result<()> {
        for len_log in 1..3 {
            test_insert_given_len(len_log)?;
        }
        Ok(())
    }
}
