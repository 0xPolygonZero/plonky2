#![allow(clippy::int_plus_one)] // Makes more sense for some inequalities below.

use alloc::vec;

use anyhow::{ensure, Result};

use crate::field::extension::Extendable;
use crate::gates::noop::NoopGate;
use crate::hash::hash_types::RichField;
use crate::iop::witness::{PartialWitness, WitnessWrite};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{
    CircuitConfig, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};

pub struct TreeRecursionNodeData<
    'a,
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof0: &'a ProofWithPublicInputs<F, C, D>,
    pub proof1: &'a ProofWithPublicInputs<F, C, D>,
    pub verifier_data0: &'a VerifierOnlyCircuitData<C, D>,
    pub verifier_data1: &'a VerifierOnlyCircuitData<C, D>,
    pub verifier_data: &'a VerifierOnlyCircuitData<C, D>,
}

pub struct TreeRecursionLeafData<
    'a,
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub inner_proof: &'a ProofWithPublicInputs<F, C, D>,
    pub inner_verifier_data: &'a VerifierOnlyCircuitData<C, D>,
    pub verifier_data: &'a VerifierOnlyCircuitData<C, D>,
}

pub struct TreeRecursionNodeTarget<const D: usize> {
    pub proof0: ProofWithPublicInputsTarget<D>,
    pub proof1: ProofWithPublicInputsTarget<D>,
    pub verifier_data0: VerifierCircuitTarget,
    pub verifier_data1: VerifierCircuitTarget,
    pub verifier_data: VerifierCircuitTarget,
}

pub struct TreeRecursionLeafTarget<const D: usize> {
    pub inner_proof: ProofWithPublicInputsTarget<D>,
    pub inner_verifier_data: VerifierCircuitTarget,
    pub verifier_data: VerifierCircuitTarget,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// WARNING: Do not register any public input before/after calling this!
    // Use requirement:
    // public inputs: [
    //   H(left_inputs, right_inputs),
    //   H(left_circuit_digest, current_circuit_digest, right_circuit_digest),
    //   current_verifier_data ]
    // Root node MUST be verified without using 'current_verifier_data' input.
    // All nodes/leaves should use the same common data.
    //
    // In this circuits:
    // 1) added two virtual inner proofs (with verifier_data as part of inputs)
    // 2) connected public inputs [0] and [1] with calculated hashes
    // 3) verified two inner proofs with their own verifier_data
    pub fn tree_recursion_node<C: GenericConfig<D, F = F>>(
        &mut self,
        common_data: &mut CommonCircuitData<F, D>,
    ) -> Result<TreeRecursionNodeTarget<D>>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let inputs_hash = self.add_virtual_hash();
        let circuit_digest_hash = self.add_virtual_hash();
        self.register_public_inputs(&inputs_hash.elements);
        self.register_public_inputs(&circuit_digest_hash.elements);

        assert!(self.verifier_data_public_input.is_none());
        self.add_verifier_data_public_inputs();
        let verifier_data = self.verifier_data_public_input.clone().unwrap();
        common_data.num_public_inputs = self.num_public_inputs();

        let proof0 = self.add_virtual_proof_with_pis(common_data);
        let proof1 = self.add_virtual_proof_with_pis(common_data);

        let verifier_data0 =
            VerifierCircuitTarget::from_slice::<F, D>(&proof0.public_inputs.clone(), common_data)?;
        let verifier_data1 =
            VerifierCircuitTarget::from_slice::<F, D>(&proof1.public_inputs.clone(), common_data)?;

        let h = self.hash_n_to_hash_no_pad::<C::Hasher>(
            [
                proof0.public_inputs[0..4].to_vec(),
                proof1.public_inputs[0..4].to_vec(),
            ]
            .concat(),
        );
        self.connect_hashes(inputs_hash, h);
        let h = self.hash_n_to_hash_no_pad::<C::Hasher>(
            [
                proof0.public_inputs[4..8].to_vec(),
                verifier_data.circuit_digest.elements.to_vec(),
                proof1.public_inputs[4..8].to_vec(),
            ]
            .concat(),
        );
        self.connect_hashes(circuit_digest_hash, h);

        self.verify_proof::<C>(&proof0, &verifier_data0, common_data);
        self.verify_proof::<C>(&proof1, &verifier_data1, common_data);

        // Make sure we have enough gates to match `common_data`.
        while self.num_gates() < (common_data.degree() / 2) {
            self.add_gate(NoopGate, vec![]);
        }
        // Make sure we have every gate to match `common_data`.
        for g in &common_data.gates {
            self.add_gate_to_gate_set(g.clone());
        }

        Ok(TreeRecursionNodeTarget {
            proof0,
            proof1,
            verifier_data0,
            verifier_data1,
            verifier_data,
        })
    }

    /// WARNING: Do not register any public input before/after calling this!
    // public inputs: [
    //   H(inner_inputs),
    //   H(current_circuit_digest, inner_circuit_digest),
    //   current_verifier_data ]
    pub fn tree_recursion_leaf<C: GenericConfig<D, F = F>>(
        &mut self,
        inner_common_data: CommonCircuitData<F, D>,
        common_data: &mut CommonCircuitData<F, D>,
    ) -> Result<TreeRecursionLeafTarget<D>>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let inputs_hash = self.add_virtual_hash();
        let circuit_digest_hash = self.add_virtual_hash();
        self.register_public_inputs(&inputs_hash.elements);
        self.register_public_inputs(&circuit_digest_hash.elements);

        assert!(self.verifier_data_public_input.is_none());
        self.add_verifier_data_public_inputs();
        let verifier_data = self.verifier_data_public_input.clone().unwrap();
        common_data.num_public_inputs = self.num_public_inputs();

        let inner_proof = self.add_virtual_proof_with_pis(&inner_common_data);
        let inner_verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self
                .add_virtual_cap(inner_common_data.config.fri_config.cap_height),
            circuit_digest: self.add_virtual_hash(),
        };

        let h = self.hash_n_to_hash_no_pad::<C::Hasher>(inner_proof.public_inputs.clone());
        self.connect_hashes(inputs_hash, h);
        let h = self.hash_n_to_hash_no_pad::<C::Hasher>(
            [
                inner_verifier_data.circuit_digest.elements,
                verifier_data.circuit_digest.elements,
            ]
            .concat(),
        );
        self.connect_hashes(circuit_digest_hash, h);

        self.verify_proof::<C>(&inner_proof, &inner_verifier_data, &inner_common_data);

        // Make sure we have enough gates to match `common_data`.
        while self.num_gates() < (common_data.degree() / 2) {
            self.add_gate(NoopGate, vec![]);
        }
        // Make sure we have every gate to match `common_data`.
        for g in &common_data.gates {
            self.add_gate_to_gate_set(g.clone());
        }

        Ok(TreeRecursionLeafTarget {
            inner_proof,
            inner_verifier_data,
            verifier_data,
        })
    }
}

/// Set the targets in a `TreeRecursionNodeTarget` to their corresponding values in a `TreeRecursionNodeData`.
pub fn set_tree_recursion_node_data_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    pw: &mut PartialWitness<F>,
    tree_recursion_data_target: &TreeRecursionNodeTarget<D>,
    tree_recursion_data: &TreeRecursionNodeData<F, C, D>,
) -> Result<()>
where
    C::Hasher: AlgebraicHasher<F>,
{
    pw.set_proof_with_pis_target(
        &tree_recursion_data_target.proof0,
        tree_recursion_data.proof0,
    );
    pw.set_proof_with_pis_target(
        &tree_recursion_data_target.proof1,
        tree_recursion_data.proof1,
    );
    pw.set_verifier_data_target(
        &tree_recursion_data_target.verifier_data0,
        tree_recursion_data.verifier_data0,
    );
    pw.set_verifier_data_target(
        &tree_recursion_data_target.verifier_data1,
        tree_recursion_data.verifier_data1,
    );
    pw.set_verifier_data_target(
        &tree_recursion_data_target.verifier_data,
        tree_recursion_data.verifier_data,
    );

    Ok(())
}

/// Set the targets in a `TreeRecursionLeafTarget` to their corresponding values in a `TreeRecursionLeafData`.
pub fn set_tree_recursion_leaf_data_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    pw: &mut PartialWitness<F>,
    tree_recursion_data_target: &TreeRecursionLeafTarget<D>,
    tree_recursion_data: &TreeRecursionLeafData<F, C, D>,
) -> Result<()>
where
    C::Hasher: AlgebraicHasher<F>,
{
    pw.set_proof_with_pis_target(
        &tree_recursion_data_target.inner_proof,
        tree_recursion_data.inner_proof,
    );
    pw.set_verifier_data_target(
        &tree_recursion_data_target.inner_verifier_data,
        tree_recursion_data.inner_verifier_data,
    );
    pw.set_verifier_data_target(
        &tree_recursion_data_target.verifier_data,
        tree_recursion_data.verifier_data,
    );

    Ok(())
}

/// Additional checks to be performed on a tree recursive proof in addition to verifying the proof.
/// Checks that the purported verifier data in the public inputs match the real verifier data.
pub fn check_tree_proof_verifier_data<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    proof: &ProofWithPublicInputs<F, C, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()>
where
    C::Hasher: AlgebraicHasher<F>,
{
    let pis = VerifierOnlyCircuitData::<C, D>::from_slice(&proof.public_inputs, common_data)?;
    ensure!(verifier_data.constants_sigmas_cap == pis.constants_sigmas_cap);
    ensure!(verifier_data.circuit_digest == pis.circuit_digest);

    Ok(())
}

// Generates `CommonCircuitData` usable for recursion.
pub fn common_data_for_recursion<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>() -> CommonCircuitData<F, D>
where
    C::Hasher: AlgebraicHasher<F>,
{
    let config = CircuitConfig::standard_recursion_config();
    let builder = CircuitBuilder::<F, D>::new(config);
    let data = builder.build::<C>();
    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);
    let proof = builder.add_virtual_proof_with_pis(&data.common);
    let verifier_data = VerifierCircuitTarget {
        constants_sigmas_cap: builder.add_virtual_cap(data.common.config.fri_config.cap_height),
        circuit_digest: builder.add_virtual_hash(),
    };
    builder.verify_proof::<C>(&proof, &verifier_data, &data.common);
    let data = builder.build::<C>();

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);
    let proof = builder.add_virtual_proof_with_pis(&data.common);
    let verifier_data = VerifierCircuitTarget {
        constants_sigmas_cap: builder.add_virtual_cap(data.common.config.fri_config.cap_height),
        circuit_digest: builder.add_virtual_hash(),
    };
    builder.verify_proof::<C>(&proof, &verifier_data, &data.common);
    while builder.num_gates() < 1 << 12 {
        builder.add_gate(NoopGate, vec![]);
    }
    builder.build::<C>().common
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::types::Field;
    use crate::gates::noop::NoopGate;
    use crate::hash::hash_types::HashOut;
    use crate::hash::hashing::hash_n_to_hash_no_pad;
    use crate::hash::poseidon::PoseidonPermutation;
    use crate::iop::witness::{PartialWitness, WitnessWrite};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::recursion::tree_recursion::{
        check_tree_proof_verifier_data, common_data_for_recursion,
        set_tree_recursion_leaf_data_target, set_tree_recursion_node_data_target,
        TreeRecursionLeafData, TreeRecursionNodeData,
    };

    #[test]
    fn test_tree_recursion() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        // create dummy proof0
        let hash0 = HashOut {
            elements: [F::ZERO, F::ZERO, F::ZERO, F::ZERO],
        };
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        for _ in 0..1_000 {
            builder.add_gate(NoopGate, vec![]);
        }
        let input_hash = builder.add_virtual_hash();
        builder.register_public_inputs(&input_hash.elements);
        let data = builder.build::<C>();
        let mut inputs = PartialWitness::new();
        inputs.set_hash_target(input_hash, hash0);
        let proof0 = data.prove(inputs)?;
        data.verify(proof0.clone())?;
        let cd0 = data.common;
        let vd0 = data.verifier_only;

        // create dummy proof1
        let hash1 = HashOut {
            elements: [F::ZERO, F::ZERO, F::ZERO, F::ONE],
        };
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        for _ in 0..2_000 {
            builder.add_gate(NoopGate, vec![]);
        }
        let input_hash = builder.add_virtual_hash();
        builder.register_public_inputs(&input_hash.elements);
        let data = builder.build::<C>();
        let mut inputs = PartialWitness::new();
        inputs.set_hash_target(input_hash, hash1);
        let proof1 = data.prove(inputs)?;
        data.verify(proof1.clone())?;
        let cd1 = data.common;
        let vd1 = data.verifier_only;

        // create dummy proof2
        let hash2 = HashOut {
            elements: [F::ZERO, F::ZERO, F::ZERO, F::TWO],
        };
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        for _ in 0..4_000 {
            builder.add_gate(NoopGate, vec![]);
        }
        let input_hash = builder.add_virtual_hash();
        builder.register_public_inputs(&input_hash.elements);
        let data = builder.build::<C>();
        let mut inputs = PartialWitness::new();
        inputs.set_hash_target(input_hash, hash2);
        let proof2 = data.prove(inputs)?;
        data.verify(proof2.clone())?;
        let cd2 = data.common;
        let vd2 = data.verifier_only;

        let mut common_data = common_data_for_recursion::<F, C, D>();
        // build leaf0
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let leaf_targets = builder.tree_recursion_leaf::<C>(cd0, &mut common_data)?;
        let data = builder.build::<C>();
        let leaf_vd0 = &data.verifier_only;
        let mut pw = PartialWitness::new();
        let leaf_data = TreeRecursionLeafData {
            inner_proof: &proof0,
            inner_verifier_data: &vd0,
            verifier_data: leaf_vd0,
        };
        set_tree_recursion_leaf_data_target(&mut pw, &leaf_targets, &leaf_data)?;
        let leaf_proof0 = data.prove(pw)?;
        check_tree_proof_verifier_data(&leaf_proof0, leaf_vd0, &common_data)
            .expect("Leaf 0 public inputs do not match its verifier data");

        // build leaf1
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let leaf_targets = builder.tree_recursion_leaf::<C>(cd1, &mut common_data)?;
        let data = builder.build::<C>();
        let leaf_vd1 = &data.verifier_only;
        let mut pw = PartialWitness::new();
        let leaf_data = TreeRecursionLeafData {
            inner_proof: &proof1,
            inner_verifier_data: &vd1,
            verifier_data: leaf_vd1,
        };
        set_tree_recursion_leaf_data_target(&mut pw, &leaf_targets, &leaf_data)?;
        let leaf_proof1 = data.prove(pw)?;
        check_tree_proof_verifier_data(&leaf_proof1, leaf_vd1, &common_data)
            .expect("Leaf 1 public inputs do not match its verifier data");

        // build leaf2
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let leaf_targets = builder.tree_recursion_leaf::<C>(cd2, &mut common_data)?;
        let data = builder.build::<C>();
        let leaf_vd2 = &data.verifier_only;
        let mut pw = PartialWitness::new();
        let leaf_data = TreeRecursionLeafData {
            inner_proof: &proof2,
            inner_verifier_data: &vd2,
            verifier_data: leaf_vd2,
        };
        set_tree_recursion_leaf_data_target(&mut pw, &leaf_targets, &leaf_data)?;
        let leaf_proof2 = data.prove(pw)?;
        check_tree_proof_verifier_data(&leaf_proof2, leaf_vd2, &common_data)
            .expect("Leaf 2 public inputs do not match its verifier data");

        // build node
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let node_targets = builder.tree_recursion_node::<C>(&mut common_data)?;
        let data = builder.build::<C>();
        let node_vd = &data.verifier_only;
        let mut pw = PartialWitness::new();
        let node_data = TreeRecursionNodeData {
            proof0: &leaf_proof0,
            proof1: &leaf_proof1,
            verifier_data0: &leaf_vd0,
            verifier_data1: &leaf_vd1,
            verifier_data: node_vd,
        };
        set_tree_recursion_node_data_target(&mut pw, &node_targets, &node_data)?;
        let node_proof = data.prove(pw)?;
        check_tree_proof_verifier_data(&node_proof, node_vd, &common_data)
            .expect("Node public inputs do not match its verifier data");

        // build root node
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let root_targets = builder.tree_recursion_node::<C>(&mut common_data)?;
        let data = builder.build::<C>();
        let root_vd = &data.verifier_only;
        let mut pw = PartialWitness::new();
        let root_data = TreeRecursionNodeData {
            proof0: &node_proof,
            proof1: &leaf_proof2,
            verifier_data0: &node_vd,
            verifier_data1: &leaf_vd2,
            verifier_data: root_vd,
        };
        set_tree_recursion_node_data_target(&mut pw, &root_targets, &root_data)?;
        let root_proof = data.prove(pw)?;
        check_tree_proof_verifier_data(&root_proof, root_vd, &common_data)
            .expect("Node public inputs do not match its verifier data");
        assert_eq!(node_vd.circuit_digest, root_vd.circuit_digest);
        assert_eq!(node_vd.constants_sigmas_cap, root_vd.constants_sigmas_cap);
        println!("{:?}", node_vd.circuit_digest.elements);

        // Verify that the proof correctly computes the input hash.
        let leaf0_input_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(&hash0.elements);
        let leaf1_input_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(&hash1.elements);
        let leaf2_input_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(&hash2.elements);
        assert_eq!(leaf0_input_hash.elements, leaf_proof0.public_inputs[0..4]);
        assert_eq!(leaf1_input_hash.elements, leaf_proof1.public_inputs[0..4]);
        let node_input_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(
            [
                leaf0_input_hash.elements.to_vec(),
                leaf1_input_hash.elements.to_vec(),
            ]
            .concat()
            .as_slice(),
        );
        assert_eq!(node_input_hash.elements, node_proof.public_inputs[0..4]);
        let root_input_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(
            [
                node_input_hash.elements.to_vec(),
                leaf2_input_hash.elements.to_vec(),
            ]
            .concat()
            .as_slice(),
        );
        assert_eq!(root_input_hash.elements, root_proof.public_inputs[0..4]);

        // Verify that the proof correctly computes the circuit hash.
        let leaf0_circuit_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(
            [
                vd0.circuit_digest.elements.to_vec(),
                leaf_vd0.circuit_digest.elements.to_vec(),
            ]
            .concat()
            .as_slice(),
        );
        assert_eq!(leaf0_circuit_hash.elements, leaf_proof0.public_inputs[4..8]);
        let leaf1_circuit_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(
            [
                vd1.circuit_digest.elements.to_vec(),
                leaf_vd1.circuit_digest.elements.to_vec(),
            ]
            .concat()
            .as_slice(),
        );
        assert_eq!(leaf1_circuit_hash.elements, leaf_proof1.public_inputs[4..8]);
        let leaf2_circuit_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(
            [
                vd2.circuit_digest.elements.to_vec(),
                leaf_vd2.circuit_digest.elements.to_vec(),
            ]
            .concat()
            .as_slice(),
        );
        assert_eq!(leaf2_circuit_hash.elements, leaf_proof2.public_inputs[4..8]);
        let node_circuit_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(
            [
                leaf0_circuit_hash.elements.to_vec(),
                node_vd.circuit_digest.elements.to_vec(),
                leaf1_circuit_hash.elements.to_vec(),
            ]
            .concat()
            .as_slice(),
        );
        assert_eq!(node_circuit_hash.elements, node_proof.public_inputs[4..8]);
        let root_circuit_hash = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(
            [
                node_circuit_hash.elements.to_vec(),
                node_vd.circuit_digest.elements.to_vec(),
                leaf2_circuit_hash.elements.to_vec(),
            ]
            .concat()
            .as_slice(),
        );
        assert_eq!(root_circuit_hash.elements, root_proof.public_inputs[4..8]);

        Ok(())
    }
}
