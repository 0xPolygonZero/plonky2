use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::random_access::RandomAccessGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Checks that a `Target` matches a vector at a non-deterministic index.
    /// Note: `index` is not range-checked.
    pub fn random_access(
        &mut self,
        access_index: Target,
        claimed_element: ExtensionTarget<D>,
        v: Vec<ExtensionTarget<D>>,
    ) {
        let gate = RandomAccessGate::new(v.len());
        let gate_index = self.add_gate(gate.clone(), vec![]);

        v.iter().enumerate().for_each(|(i, &val)| {
            self.connect_extension(
                val,
                ExtensionTarget::from_range(gate_index, gate.wires_list_item(i)),
            );
        });
        self.connect(
            access_index,
            Target::wire(gate_index, gate.wire_access_index()),
        );
        self.connect_extension(
            claimed_element,
            ExtensionTarget::from_range(gate_index, gate.wires_claimed_element()),
        );
    }

    /// Like `random_access`, but first pads `v` to a given minimum length. This can help to avoid
    /// having multiple `RandomAccessGate`s with different sizes.
    pub fn random_access_padded(
        &mut self,
        access_index: Target,
        claimed_element: ExtensionTarget<D>,
        mut v: Vec<ExtensionTarget<D>>,
        min_length: usize,
    ) {
        let zero = self.zero_extension();
        if v.len() < min_length {
            v.resize(8, zero);
        }
        self.random_access(access_index, claimed_element, v);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_random_access_given_len(len_log: usize) -> Result<()> {
        type F = CrandallField;
        type FF = QuarticExtension<CrandallField>;
        let len = 1 << len_log;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let vec = FF::rand_vec(len);
        let v: Vec<_> = vec.iter().map(|x| builder.constant_extension(*x)).collect();

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let elem = builder.constant_extension(vec[i]);
            builder.random_access(it, elem, v.clone());
        }

        let data = builder.build();
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
