use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
use crate::cpu::columns::CpuColumnsView;
use crate::logic;
use crate::witness::memory::MemoryOp;

type LogicRow<T> = [T; logic::columns::NUM_COLUMNS];
type ArithmeticRow<T> = [T; NUM_ARITH_COLUMNS];

struct Traces<T: Copy> {
    pub cpu: Vec<CpuColumnsView<T>>,
    pub logic: Vec<LogicRow<T>>,
    pub arithmetic: Vec<ArithmeticRow<T>>,
    pub memory: Vec<MemoryOp>,
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

    pub fn append(&mut self, other: &mut Self) {
        self.cpu.append(&mut other.cpu);
        self.logic.append(&mut other.logic);
        self.arithmetic.append(&mut other.arithmetic);
        self.memory.append(&mut other.memory);
    }
}
