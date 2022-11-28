use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
use crate::cpu::columns::CpuColumnsView;
use crate::logic;
use crate::witness::memory::MemoryOp;

type LogicRow<T> = [T; logic::columns::NUM_COLUMNS];
type ArithmeticRow<T> = [T; NUM_ARITH_COLUMNS];

#[derive(Clone, Copy, Debug)]
pub struct TraceCheckpoint {
    pub(self) cpu_len: usize,
    pub(self) logic_len: usize,
    pub(self) arithmetic_len: usize,
    pub(self) memory_len: usize,
}

#[derive(Clone, Debug)]
pub struct Traces<T: Copy> {
    cpu: Vec<CpuColumnsView<T>>,
    logic: Vec<LogicRow<T>>,
    arithmetic: Vec<ArithmeticRow<T>>,
    memory: Vec<MemoryOp>,
}

impl<T: Copy> Traces<T> {
    pub fn new() -> Self {
        Traces {
            cpu: vec![],
            logic: vec![],
            arithmetic: vec![],
            memory: vec![],
        }
    }

    pub fn checkpoint(&self) -> TraceCheckpoint {
        TraceCheckpoint {
            cpu_len: self.cpu.len(),
            logic_len: self.logic.len(),
            arithmetic_len: self.arithmetic.len(),
            memory_len: self.memory.len(),
        }
    }

    pub fn rollback(&mut self, checkpoint: TraceCheckpoint) {
        self.cpu.truncate(checkpoint.cpu_len);
        self.logic.truncate(checkpoint.logic_len);
        self.arithmetic.truncate(checkpoint.arithmetic_len);
        self.memory.truncate(checkpoint.memory_len);
    }

    pub fn push_cpu(&mut self, val: CpuColumnsView<T>) {
        self.cpu.push(val);
    }

    pub fn push_logic(&mut self, val: LogicRow<T>) {
        self.logic.push(val);
    }

    pub fn push_arithmetic(&mut self, val: ArithmeticRow<T>) {
        self.arithmetic.push(val);
    }

    pub fn push_memory(&mut self, val: MemoryOp) {
        self.memory.push(val);
    }

    pub fn clock(&self) -> usize {
        self.cpu.len()
    }
}

impl<T: Copy> Default for Traces<T> {
    fn default() -> Self {
        Self::new()
    }
}
