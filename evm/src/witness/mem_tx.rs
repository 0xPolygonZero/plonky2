use crate::witness::memory::{MemoryOp, MemoryOpKind, MemoryState};

pub fn apply_mem_ops(state: &mut MemoryState, mut ops: Vec<MemoryOp>) {
    ops.sort_unstable_by_key(|mem_op| mem_op.timestamp);

    for op in ops {
        let MemoryOp { address, op, .. } = op;
        if let MemoryOpKind::Write(val) = op {
            state.set(address, val);
        }
    }
}
