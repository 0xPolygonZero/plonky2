fn to_byte_checked(n: U256) -> u8 {
    let res = n.byte(0);
    assert_eq!(n, res.into());
    res
}

fn to_bits<F: Field>(n: u8) -> [F; 8] {
    let mut res = [F::ZERO; 8];
    for (i, bit) in res.iter_mut().enumerate() {
        *bit = F::from_bool(n & (1 << i) != 0);
    }
    res
}

fn decode(state: &State, row: &mut CpuRow) -> (Operation, MemoryLog) {
    let code_context = if state.is_kernel {
        KERNEL_CONTEXT
    } else {
        state.context
    };
    row.code_context = F::from_canonical_u32(code_context);

    let address = (context, Segment::Code as u32, state.program_counter);
    let mem_contents, mem_log = state.mem_read_with_log(address);
    let opcode = to_byte_checked(mem_contents);
    row.opcode_bits = to_bits(address);

    let operation = match opcode {
        0x01 => Operation::NotImplemented,
        0x02 => Operation::NotImplemented,
        0x03 => Operation::NotImplemented,
        0x04 => Operation::NotImplemented,
        0x06 => Operation::NotImplemented,
        0x08 => Operation::NotImplemented,
        0x09 => Operation::NotImplemented,
        0x0c => Operation::NotImplemented,
        0x0d => Operation::NotImplemented,
        0x0e => Operation::NotImplemented,
        0x10 => Operation::NotImplemented,
        0x11 => Operation::NotImplemented,
        0x14 => Operation::Eq,
        0x15 => Operation::Iszero,
        0x16 => Operation::BinaryLogic(BinaryLogicOp::And),
        0x17 => Operation::BinaryLogic(BinaryLogicOp::Or),
        0x18 => Operation::BinaryLogic(BinaryLogicOp::Xor),
        0x19 => Operation::Not,
        0x1a => Operation::Byte,
        0x1b => Operation::NotImplemented,
        0x1c => Operation::NotImplemented,
        0x21 => Operation::NotImplemented,
        0x49 => Operation::NotImplemented,
        0x50 => Operation::NotImplemented,
        0x56 => Operation::Jump,
        0x57 => Operation::Jumpi,
        0x58 => Operation::NotImplemented,
        0x5a => Operation::NotImplemented,
        0x5b => Operation::NotImplemented,
        0x5c => Operation::NotImplemented,
        0x5d => Operation::NotImplemented,
        0x5e => Operation::NotImplemented,
        0x5f => Operation::NotImplemented,
        0x60..0x7f => Operation::NotImplemented,
        0x80..0x8f => Operation::Dup(opcode & 0xf),
        0x90..0x9f => Operation::Swap(opcode & 0xf),
        0xf6 => Operation::NotImplemented,
        0xf7 => Operation::NotImplemented,
        0xf8 => Operation::NotImplemented,
        0xf9 => Operation::ExitKernel,
        0xfb => Operation::NotImplemented,
        0xfc => Operation::NotImplemented,
        _ => Operation::Syscall,
    }
}

fn op_result() {
    match op {

    }
}

pub fn transition<F: Field>(state: State) -> (State, Traces<T>) {
    let mut current_row: CpuColumnsView<F> = [F::ZERO; NUM_CPU_COLUMNS].into();

    current_row.is_cpu_cycle = F::ONE;
    
    let (op, code_mem_log) = decode(&state, &mut current_row);
}
