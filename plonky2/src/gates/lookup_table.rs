#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::sync::Arc;

use anyhow::Result;
use itertools::Itertools;
use keccak_hash::keccak;

use crate::field::extension::Extendable;
use crate::field::packed::PackedField;
use crate::gates::gate::Gate;
use crate::gates::packed_util::PackedEvaluableBase;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGeneratorRef};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, WitnessWrite};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData};
use crate::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use crate::util::serialization::{Buffer, IoResult, Read, Write};

pub type LookupTable = Arc<Vec<(u16, u16)>>;

/// A gate which stores the set of (input, output) value pairs of a lookup table, and their multiplicities.
#[derive(Debug, Clone)]
pub struct LookupTableGate {
    /// Number of lookup entries per gate.
    pub num_slots: usize,
    /// Lookup table associated to the gate.
    pub lut: LookupTable,
    /// The Keccak hash of the lookup table.
    lut_hash: [u8; 32],
    /// First row of the lookup table.
    last_lut_row: usize,
}

impl LookupTableGate {
    pub fn new_from_table(config: &CircuitConfig, lut: LookupTable, last_lut_row: usize) -> Self {
        let table_bytes = lut
            .iter()
            .flat_map(|(input, output)| [input.to_le_bytes(), output.to_le_bytes()].concat())
            .collect_vec();

        Self {
            num_slots: Self::num_slots(config),
            lut,
            lut_hash: keccak(table_bytes).0,
            last_lut_row,
        }
    }

    pub(crate) const fn num_slots(config: &CircuitConfig) -> usize {
        let wires_per_entry = 3;
        config.num_routed_wires / wires_per_entry
    }

    /// Wire for the looked input.
    pub const fn wire_ith_looked_inp(i: usize) -> usize {
        3 * i
    }

    // Wire for the looked output.
    pub const fn wire_ith_looked_out(i: usize) -> usize {
        3 * i + 1
    }

    /// Wire for the multiplicity. Set after the trace has been generated.
    pub const fn wire_ith_multiplicity(i: usize) -> usize {
        3 * i + 2
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for LookupTableGate {
    fn id(&self) -> String {
        // Custom implementation to not have the entire lookup table
        format!(
            "LookupTableGate {{num_slots: {}, lut_hash: {:?}, last_lut_row: {}}}",
            self.num_slots, self.lut_hash, self.last_lut_row
        )
    }

    fn serialize(&self, dst: &mut Vec<u8>, common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.num_slots)?;
        dst.write_usize(self.last_lut_row)?;
        for (i, lut) in common_data.luts.iter().enumerate() {
            if lut == &self.lut {
                dst.write_usize(i)?;
                return dst.write_all(&self.lut_hash);
            }
        }

        panic!("The associated lookup table couldn't be found.")
    }

    fn deserialize(src: &mut Buffer, common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let num_slots = src.read_usize()?;
        let last_lut_row = src.read_usize()?;
        let lut_index = src.read_usize()?;
        let mut lut_hash = [0u8; 32];
        src.read_exact(&mut lut_hash)?;

        Ok(Self {
            num_slots,
            lut: common_data.luts[lut_index].clone(),
            lut_hash,
            last_lut_row,
        })
    }

    fn eval_unfiltered(&self, _vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        // No main trace constraints for the lookup table.
        vec![]
    }

    fn eval_unfiltered_base_one(
        &self,
        _vars: EvaluationVarsBase<F>,
        _yield_constr: StridedConstraintConsumer<F>,
    ) {
        panic!("use eval_unfiltered_base_packed instead");
    }

    fn eval_unfiltered_base_batch(&self, vars_base: EvaluationVarsBaseBatch<F>) -> Vec<F> {
        self.eval_unfiltered_base_batch_packed(vars_base)
    }

    fn eval_unfiltered_circuit(
        &self,
        _builder: &mut CircuitBuilder<F, D>,
        _vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        // No main trace constraints for the lookup table.
        vec![]
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<WitnessGeneratorRef<F, D>> {
        (0..self.num_slots)
            .map(|i| {
                WitnessGeneratorRef::new(
                    LookupTableGenerator {
                        row,
                        lut: self.lut.clone(),
                        slot_nb: i,
                        num_slots: self.num_slots,
                        last_lut_row: self.last_lut_row,
                    }
                    .adapter(),
                )
            })
            .collect()
    }

    fn num_wires(&self) -> usize {
        self.num_slots * 3
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        0
    }

    fn num_constraints(&self) -> usize {
        0
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for LookupTableGate {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        _vars: EvaluationVarsBasePacked<P>,
        mut _yield_constr: StridedConstraintConsumer<P>,
    ) {
    }
}

#[derive(Clone, Debug, Default)]
pub struct LookupTableGenerator {
    row: usize,
    lut: LookupTable,
    slot_nb: usize,
    num_slots: usize,
    last_lut_row: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D> for LookupTableGenerator {
    fn id(&self) -> String {
        "LookupTableGenerator".to_string()
    }

    fn dependencies(&self) -> Vec<Target> {
        vec![]
    }

    fn run_once(
        &self,
        _witness: &PartitionWitness<F>,
        out_buffer: &mut GeneratedValues<F>,
    ) -> Result<()> {
        let first_row = self.last_lut_row + self.lut.len().div_ceil(self.num_slots) - 1;
        let slot = (first_row - self.row) * self.num_slots + self.slot_nb;

        let slot_input_target =
            Target::wire(self.row, LookupTableGate::wire_ith_looked_inp(self.slot_nb));
        let slot_output_target =
            Target::wire(self.row, LookupTableGate::wire_ith_looked_out(self.slot_nb));

        if slot < self.lut.len() {
            let (input, output) = self.lut[slot];
            out_buffer.set_target(slot_input_target, F::from_canonical_usize(input as usize))?;
            out_buffer.set_target(slot_output_target, F::from_canonical_usize(output as usize))
        } else {
            // Pad with first element in the LUT.
            assert!(!self.lut.is_empty(), "Empty LUTs are not supported.");
            let (input, output) = self.lut[0];
            out_buffer.set_target(slot_input_target, F::from_canonical_usize(input as usize))?;
            out_buffer.set_target(slot_output_target, F::from_canonical_usize(output as usize))
        }
    }

    fn serialize(&self, dst: &mut Vec<u8>, common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.row)?;
        dst.write_usize(self.slot_nb)?;
        dst.write_usize(self.num_slots)?;
        dst.write_usize(self.last_lut_row)?;
        for (i, lut) in common_data.luts.iter().enumerate() {
            if lut == &self.lut {
                return dst.write_usize(i);
            }
        }

        panic!("The associated lookup table couldn't be found.")
    }

    fn deserialize(src: &mut Buffer, common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let row = src.read_usize()?;
        let slot_nb = src.read_usize()?;
        let num_slots = src.read_usize()?;
        let last_lut_row = src.read_usize()?;
        let lut_index = src.read_usize()?;

        Ok(Self {
            row,
            lut: common_data.luts[lut_index].clone(),
            slot_nb,
            num_slots,
            last_lut_row,
        })
    }
}
