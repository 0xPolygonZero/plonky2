use std::mem::size_of;

use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

use crate::all_stark::{AllStark, NUM_TABLES};
use crate::config::StarkConfig;
use crate::cpu::columns::CpuColumnsView;
use crate::keccak_sponge::columns::KECCAK_WIDTH_BYTES;
use crate::keccak_sponge::keccak_sponge_stark::KeccakSpongeOp;
use crate::util::trace_rows_to_poly_values;
use crate::witness::memory::MemoryOp;
use crate::{arithmetic, keccak, logic};

#[derive(Clone, Copy, Debug)]
pub struct TraceCheckpoint {
    pub(self) cpu_len: usize,
    pub(self) keccak_len: usize,
    pub(self) keccak_sponge_len: usize,
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
            keccak_sponge_ops: vec![],
        }
    }

    pub fn checkpoint(&self) -> TraceCheckpoint {
        TraceCheckpoint {
            cpu_len: self.cpu.len(),
            keccak_len: self.keccak_inputs.len(),
            keccak_sponge_len: self.keccak_sponge_ops.len(),
            logic_len: self.logic_ops.len(),
            arithmetic_len: self.arithmetic.len(),
            memory_len: self.memory_ops.len(),
        }
    }

    pub fn rollback(&mut self, checkpoint: TraceCheckpoint) {
        self.cpu.truncate(checkpoint.cpu_len);
        self.keccak_inputs.truncate(checkpoint.keccak_len);
        self.keccak_sponge_ops
            .truncate(checkpoint.keccak_sponge_len);
        self.logic_ops.truncate(checkpoint.logic_len);
        self.arithmetic.truncate(checkpoint.arithmetic_len);
        self.memory_ops.truncate(checkpoint.memory_len);
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

    pub fn push_keccak(&mut self, input: [u64; keccak::keccak_stark::NUM_INPUTS]) {
        self.keccak_inputs.push(input);
    }

    pub fn push_keccak_bytes(&mut self, input: [u8; KECCAK_WIDTH_BYTES]) {
        let chunks = input
            .chunks(size_of::<u64>())
            .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
            .collect_vec()
            .try_into()
            .unwrap();
        self.push_keccak(chunks);
    }

    pub fn push_keccak_sponge(&mut self, op: KeccakSpongeOp) {
        self.keccak_sponge_ops.push(op);
    }

    pub fn clock(&self) -> usize {
        self.cpu.len()
    }

    pub fn into_tables<const D: usize>(
        self,
        all_stark: &AllStark<T, D>,
        config: &StarkConfig,
        timing: &mut TimingTree,
    ) -> [Vec<PolynomialValues<T>>; NUM_TABLES]
    where
        T: RichField + Extendable<D>,
    {
        let cap_elements = config.fri_config.num_cap_elements();
        let Traces {
            cpu,
            logic_ops,
            arithmetic: _, // TODO
            memory_ops,
            keccak_inputs,
            keccak_sponge_ops,
        } = self;

        let cpu_rows = cpu.into_iter().map(|x| x.into()).collect();
        let cpu_trace = trace_rows_to_poly_values(cpu_rows);
        let keccak_trace = timed!(
            timing,
            "generate Keccak trace",
            all_stark
                .keccak_stark
                .generate_trace(keccak_inputs, cap_elements, timing)
        );
        let keccak_sponge_trace = timed!(
            timing,
            "generate Keccak sponge trace",
            all_stark
                .keccak_sponge_stark
                .generate_trace(keccak_sponge_ops, cap_elements, timing)
        );
        let logic_trace = timed!(
            timing,
            "generate logic trace",
            all_stark
                .logic_stark
                .generate_trace(logic_ops, cap_elements, timing)
        );
        let memory_trace = timed!(
            timing,
            "generate memory trace",
            all_stark.memory_stark.generate_trace(memory_ops, timing)
        );

        [
            cpu_trace,
            keccak_trace,
            keccak_sponge_trace,
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
