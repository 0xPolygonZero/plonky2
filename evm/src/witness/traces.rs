use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::util::timing::TimingTree;

use crate::all_stark::{AllStark, NUM_TABLES};
use crate::config::StarkConfig;
use crate::cpu::columns::CpuColumnsView;
use crate::keccak_memory::keccak_memory_stark::KeccakMemoryOp;
use crate::keccak_sponge::keccak_sponge_stark::KeccakSpongeOp;
use crate::util::trace_rows_to_poly_values;
use crate::witness::memory::MemoryOp;
use crate::{arithmetic, keccak, logic};

#[derive(Clone, Copy, Debug)]
pub struct TraceCheckpoint {
    pub(self) cpu_len: usize,
    pub(self) logic_len: usize,
    pub(self) arithmetic_len: usize,
    pub(self) memory_len: usize,
}

#[derive(Debug)]
pub(crate) struct Traces<T: Copy> {
    pub(crate) cpu: Vec<CpuColumnsView<T>>,
    pub(crate) logic_ops: Vec<logic::Operation>,
    pub(crate) arithmetic: Vec<arithmetic::Operation>,
    pub(crate) memory_ops: Vec<MemoryOp>,
    pub(crate) keccak_inputs: Vec<[u64; keccak::keccak_stark::NUM_INPUTS]>,
    pub(crate) keccak_memory_inputs: Vec<KeccakMemoryOp>,
    pub(crate) keccak_sponge_ops: Vec<KeccakSpongeOp>,
}

impl<T: Copy> Traces<T> {
    pub fn new() -> Self {
        Traces {
            cpu: vec![],
            logic_ops: vec![],
            arithmetic: vec![],
            memory_ops: vec![],
            keccak_inputs: vec![],
            keccak_memory_inputs: vec![],
            keccak_sponge_ops: vec![],
        }
    }

    pub fn checkpoint(&self) -> TraceCheckpoint {
        TraceCheckpoint {
            cpu_len: self.cpu.len(),
            logic_len: self.logic_ops.len(),
            arithmetic_len: self.arithmetic.len(),
            memory_len: self.memory_ops.len(),
            // TODO others
        }
    }

    pub fn rollback(&mut self, checkpoint: TraceCheckpoint) {
        self.cpu.truncate(checkpoint.cpu_len);
        self.logic_ops.truncate(checkpoint.logic_len);
        self.arithmetic.truncate(checkpoint.arithmetic_len);
        self.memory_ops.truncate(checkpoint.memory_len);
        // TODO others
    }

    pub fn mem_ops_since(&self, checkpoint: TraceCheckpoint) -> &[MemoryOp] {
        &self.memory_ops[checkpoint.memory_len..]
    }

    pub fn push_cpu(&mut self, val: CpuColumnsView<T>) {
        self.cpu.push(val);
    }

    pub fn push_logic(&mut self, op: logic::Operation) {
        self.logic_ops.push(op);
    }

    pub fn push_arithmetic(&mut self, op: arithmetic::Operation) {
        self.arithmetic.push(op);
    }

    pub fn push_memory(&mut self, op: MemoryOp) {
        self.memory_ops.push(op);
    }

    pub fn push_keccak_sponge(&mut self, op: KeccakSpongeOp) {
        self.keccak_sponge_ops.push(op);
    }

    pub fn clock(&self) -> usize {
        self.cpu.len()
    }

    pub fn to_tables<const D: usize>(
        self,
        all_stark: &AllStark<T, D>,
        config: &StarkConfig,
        timing: &mut TimingTree,
    ) -> [Vec<PolynomialValues<T>>; NUM_TABLES]
    where
        T: RichField + Extendable<D>,
    {
        let Traces {
            cpu,
            logic_ops,
            arithmetic,
            memory_ops,
            keccak_inputs,
            keccak_memory_inputs,
            keccak_sponge_ops,
        } = self;

        let cpu_rows = cpu.into_iter().map(|x| x.into()).collect();
        let cpu_trace = trace_rows_to_poly_values(cpu_rows);
        let keccak_trace = all_stark.keccak_stark.generate_trace(keccak_inputs, timing);
        let keccak_memory_trace = all_stark.keccak_memory_stark.generate_trace(
            keccak_memory_inputs,
            config.fri_config.num_cap_elements(),
            timing,
        );
        let logic_trace = all_stark.logic_stark.generate_trace(logic_ops, timing);
        let memory_trace = all_stark.memory_stark.generate_trace(memory_ops, timing);

        [
            cpu_trace,
            keccak_trace,
            keccak_memory_trace,
            logic_trace,
            memory_trace,
        ]
    }
}

impl<T: Copy> Default for Traces<T> {
    fn default() -> Self {
        Self::new()
    }
}
