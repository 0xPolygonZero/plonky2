use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::usize;

use super::lookup_table::LookupTable;
use crate::field::extension::Extendable;
use crate::field::packed::PackedField;
use crate::gates::gate::Gate;
use crate::gates::packed_util::PackedEvaluableBase;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGeneratorRef};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData};
use crate::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use crate::util::serialization::{Buffer, IoResult, Read, Write};

pub type Lookup = Vec<(Target, Target)>;

/// A gate which stores (input, output) lookup pairs made elsewhere in the trace. It doesn't check any constraints itself.
#[derive(Debug, Clone)]
pub struct LookupGate {
    /// Number of lookups per gate.
    pub num_slots: usize,
    /// LUT associated to the gate.
    lut: LookupTable,
}

impl LookupGate {
    pub fn new_from_table(config: &CircuitConfig, lut: LookupTable) -> Self {
        Self {
            num_slots: Self::num_slots(config),
            lut,
        }
    }
    pub(crate) fn num_slots(config: &CircuitConfig) -> usize {
        let wires_per_lookup = 2;
        config.num_routed_wires / wires_per_lookup
    }

    pub fn wire_ith_looking_inp(i: usize) -> usize {
        2 * i
    }

    pub fn wire_ith_looking_out(i: usize) -> usize {
        2 * i + 1
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for LookupGate {
    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn serialize(&self, dst: &mut Vec<u8>) -> IoResult<()> {
        dst.write_usize(self.num_slots)?;
        dst.write_lut(&self.lut)
    }

    fn deserialize(src: &mut Buffer, _cd: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let num_slots = src.read_usize()?;
        let lut = src.read_lut()?;

        Ok(Self {
            num_slots,
            lut: Arc::new(lut),
        })
    }

    fn eval_unfiltered(&self, _vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        // No main trace constraints for lookups.
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
        // No main trace constraints for lookups.
        vec![]
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<WitnessGeneratorRef<F, D>> {
        (0..self.num_slots)
            .map(|i| {
                WitnessGeneratorRef::new(
                    LookupGenerator {
                        row,
                        lut: self.lut.clone(),
                        slot_nb: i,
                    }
                    .adapter(),
                )
            })
            .collect()
    }

    fn num_wires(&self) -> usize {
        self.num_slots * 2
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

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for LookupGate {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        _vars: EvaluationVarsBasePacked<P>,
        mut _yield_constr: StridedConstraintConsumer<P>,
    ) {
    }
}

#[derive(Clone, Debug, Default)]
pub struct LookupGenerator {
    row: usize,
    lut: LookupTable,
    slot_nb: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D> for LookupGenerator {
    fn id(&self) -> String {
        "LookupGenerator".to_string()
    }

    fn dependencies(&self) -> Vec<Target> {
        vec![Target::wire(
            self.row,
            LookupGate::wire_ith_looking_inp(self.slot_nb),
        )]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let get_wire = |wire: usize| -> F { witness.get_target(Target::wire(self.row, wire)) };

        let input_val = get_wire(LookupGate::wire_ith_looking_inp(self.slot_nb));
        let output_val = if input_val
            == F::from_canonical_u16(self.lut[input_val.to_canonical_u64() as usize].0)
        {
            F::from_canonical_u16(self.lut[input_val.to_canonical_u64() as usize].1)
        } else {
            let mut cur_idx = 0;
            while input_val != F::from_canonical_u16(self.lut[cur_idx].0)
                && cur_idx < self.lut.len()
            {
                cur_idx += 1;
            }
            assert!(cur_idx < self.lut.len(), "Incorrect input value provided");
            F::from_canonical_u16(self.lut[cur_idx].1)
        };

        let out_wire = Target::wire(self.row, LookupGate::wire_ith_looking_out(self.slot_nb));
        out_buffer.set_target(out_wire, output_val);
    }

    fn serialize(&self, dst: &mut Vec<u8>) -> IoResult<()> {
        dst.write_usize(self.row)?;
        dst.write_lut(&self.lut)?;
        dst.write_usize(self.slot_nb)
    }

    fn deserialize(src: &mut Buffer, _cd: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let row = src.read_usize()?;
        let lut = src.read_lut()?;
        let slot_nb = src.read_usize()?;

        Ok(Self {
            row,
            lut: Arc::new(lut),
            slot_nb,
        })
    }
}
