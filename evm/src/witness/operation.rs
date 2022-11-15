use crate::cpu::kernel::aggregator::KERNEL;

enum Operation {
    Dup(u8),
    Swap(u8),
    Iszero,
    Not,
    Jump(JumpOp),
    Syscall(u8),
    Eq,
    ExitKernel,
    BinaryLogic(BinaryLogicOp),
    NotImplemented,
}


enum JumpOp {
    Jump,
    Jumpi,
}

enum BinaryLogicOp {
    And,
    Or,
    Xor,
}

impl BinaryLogicOp {
    fn result(&self, a: U256, b: U256) -> U256 {
        match self {
            BinaryLogicOp::And => a & b,
            BinaryLogicOp::Or => a | b,
            BinaryLogicOp::Xor => a ^ b,
        }
    }
}



fn make_logic_row<F>(op: BinaryLogicOp, in0: U256, in1: U256, result: U256) -> [F; logic::columns::NUM_COLUMNS] {
    let mut row = [F::ZERO; logic::columns::NUM_COLUMNS];
    row[match op {
        BinaryLogicOp::And => logic::columns::IS_AND,
        BinaryLogicOp::Or => logic::columns::IS_OR,
        BinaryLogicOp::Xor => logic::columns::IS_XOR,
    }] = F::ONE;
    for i in 0..256 {
        row[logic::columns::INPUT0[i]] = F::from_bool(in0.bit(i));
        row[logic::columns::INPUT1[i]] = F::from_bool(in1.bit(i));
    }
    let result_limbs: &[u64] = result.as_ref();
    for (i, &limb) in result_limbs.iter().enumerate() {
        row[logic::columns::RESULT[2 * i]] = F::from_canonical_u32(limb as u32);
        row[logic::columns::RESULT[2 * i + 1]] = F::from_canonical_u32((limb >> 32) as u32);
    }
    row
}


fn generate_binary_logic_op<F>(op: BinaryLogicOp, state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let ([in0, in1], logs_in) = state.pop_stack_with_log::<2>()?;
    let result = op.result(in0, in1);
    let log_out = state.push_stack_with_log(result)?;

    traces.logic.append(make_logic_row(op, in0, in1, result));
    traces.memory.extend(logs_in);
    traces.memory.append(log_out);
}

fn generate_dup<F>(n: u8, state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let other_addr_lo = state.stack_len.sub_checked(1 + (n as usize)).ok_or(ProgramError::StackUnderflow)?;
    let other_addr = (state.context, Segment::Stack as u32, other_addr_lo);

    let (val, log_in) = state.mem_read_with_log(MemoryChannel::GeneralPurpose(0), other_addr);
    let log_out = state.push_stack_with_log(val)?;

    traces.memory.extend([log_in, log_out]);
}

fn generate_swap<F>(n: u8, state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let other_addr_lo = state.stack_len.sub_checked(2 + (n as usize)).ok_or(ProgramError::StackUnderflow)?;
    let other_addr = (state.context, Segment::Stack as u32, other_addr_lo);

    let ([in0], [log_in0]) = state.pop_stack_with_log::<1>()?;
    let (in1, log_in1) = state.mem_read_with_log(MemoryChannel::GeneralPurpose(1), other_addr);
    let log_out0 = state.mem_write_with_log(MemoryChannel::GeneralPurpose(NUM_GP_CHANNELS - 2), other_addr, in0);
    let log_out1 = state.push_stack_with_log(in1)?;

    traces.memory.extend([log_in0, log_in1, log_out0, log_out1]);
}

fn generate_not<F>(state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let ([x], [log_in]) = state.pop_stack_with_log::<1>()?;
    let result = !x;
    let log_out = state.push_stack_with_log(result)?;
    
    traces.memory.append(log_in);
    traces.memory.append(log_out);
}

fn generate_iszero<F>(state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let ([x], [log_in]) = state.pop_stack_with_log::<1>()?;
    let is_zero = state.is_zero();
    let result = is_zero.into::<u64>().into::<U256>();
    let log_out = state.push_stack_with_log(result)?;

    generate_pinv_diff(x, U256::zero(), row);

    traces.memory.append(log_in);
    traces.memory.append(log_out);
}

fn generate_jump<F>(op: JumpOp, state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    todo!();
}

fn generate_syscall<F>(opcode: u8, state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let handler_jumptable_addr = KERNEL.global_labels["syscall_jumptable"] as u32;
    let handler_addr_addr = handler_jumptable_addr + (opcode as u32);
    let (handler_addr0, in_log0) = state.mem_read_with_log(MemoryChannel::GeneralPurpose(0), (0, Segment::Code as u32, handler_addr_addr));
    let (handler_addr1, in_log1) = state.mem_read_with_log(MemoryChannel::GeneralPurpose(1), (0, Segment::Code as u32, handler_addr_addr + 1));
    let (handler_addr2, in_log2) = state.mem_read_with_log(MemoryChannel::GeneralPurpose(2), (0, Segment::Code as u32, handler_addr_addr + 2));

    let handler_addr = (handler_addr0 << 16) + (handler_addr1 << 8) + handler_addr2;
    let new_program_counter = handler_addr.as_u32();

    let syscall_info = state.program_counter.into::<U256>() + (state.is_kernel_mode.into::<u64>.into::<U256> << 32);
    let log_out = state.push_stack_with_log(syscall_info)?;

    state.program_counter = new_program_counter;
    state.is_kernel = true;
}

fn generate_eq<F>(state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let ([x0, x1], logs_in) = state.pop_stack_with_log::<1>()?;
    let equal = x0 == x1;
    let result = equal.into::<u64>().into::<U256>();
    let log_out = state.push_stack_with_log(result)?;

    generate_pinv_diff(x0, x1, row);

    traces.memory.extend(logs_in);
    traces.memory.append(log_out);
}

fn generate_exit_kernel<F>(state: &mut State, row: &mut CpuRow, traces: &mut Traces<T>) -> Result<(), ProgramError> {
    let ([kexit_info], [log_in]) = state.pop_stack_with_log::<1>()?;
    let kexit_info_u64: &[u64; 4] = kexit_info.as_ref();
    let program_counter = kexit_info_u64[0] as u32;
    let is_kernel_mode_val = (kexit_info_u64[1] >> 32) as u32
    assert!(is_kernel_mode_val == 0 || is_kernel_mode_val == 1);
    let is_kernel_mode = is_kernel_mode_val != 0;

    state.program_counter = program_counter;
    state.is_kernel = is_kernel_mode;
}
