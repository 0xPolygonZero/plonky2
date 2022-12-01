use plonky2::field::types::Field;

use crate::cpu::columns::CpuColumnsView;
use crate::memory::segments::Segment;
use crate::witness::errors::ProgramError;
use crate::witness::memory::{MemoryAddress, MemoryState};
use crate::witness::operation::{
    generate_binary_logic_op, generate_dup, generate_eq, generate_exit_kernel, generate_iszero,
    generate_not, generate_push, generate_swap, generate_syscall, Operation,
};
use crate::witness::state::RegistersState;
use crate::witness::traces::Traces;
use crate::witness::util::mem_read_code_with_log_and_fill;
use crate::{arithmetic, logic};

const KERNEL_CONTEXT: usize = 0;

fn read_code_memory<F: Field>(
    registers_state: RegistersState,
    memory_state: &MemoryState,
    traces: &mut Traces<F>,
    row: &mut CpuColumnsView<F>,
) -> u8 {
    let code_context = if registers_state.is_kernel {
        KERNEL_CONTEXT
    } else {
        registers_state.context
    };
    row.code_context = F::from_canonical_usize(code_context);

    let address = MemoryAddress::new(code_context, Segment::Code, registers_state.program_counter);
    let (opcode, mem_log) = mem_read_code_with_log_and_fill(address, memory_state, traces, row);

    traces.push_memory(mem_log);

    opcode
}

fn decode(registers_state: RegistersState, opcode: u8) -> Result<Operation, ProgramError> {
    match (opcode, registers_state.is_kernel) {
        (0x00, _) => Ok(Operation::Syscall(opcode)),
        (0x01, _) => Ok(Operation::NotImplemented),
        (0x02, _) => Ok(Operation::NotImplemented),
        (0x03, _) => Ok(Operation::NotImplemented),
        (0x04, _) => Ok(Operation::NotImplemented),
        (0x05, _) => Ok(Operation::Syscall(opcode)),
        (0x06, _) => Ok(Operation::NotImplemented),
        (0x07, _) => Ok(Operation::Syscall(opcode)),
        (0x08, _) => Ok(Operation::NotImplemented),
        (0x09, _) => Ok(Operation::NotImplemented),
        (0x0a, _) => Ok(Operation::Syscall(opcode)),
        (0x0b, _) => Ok(Operation::Syscall(opcode)),
        (0x0c, true) => Ok(Operation::NotImplemented),
        (0x0d, true) => Ok(Operation::NotImplemented),
        (0x0e, true) => Ok(Operation::NotImplemented),
        (0x10, _) => Ok(Operation::NotImplemented),
        (0x11, _) => Ok(Operation::NotImplemented),
        (0x12, _) => Ok(Operation::Syscall(opcode)),
        (0x13, _) => Ok(Operation::Syscall(opcode)),
        (0x14, _) => Ok(Operation::Eq),
        (0x15, _) => Ok(Operation::Iszero),
        (0x16, _) => Ok(Operation::BinaryLogic(logic::Op::And)),
        (0x17, _) => Ok(Operation::BinaryLogic(logic::Op::Or)),
        (0x18, _) => Ok(Operation::BinaryLogic(logic::Op::Xor)),
        (0x19, _) => Ok(Operation::Not),
        (0x1a, _) => Ok(Operation::NotImplemented),
        (0x1b, _) => Ok(Operation::NotImplemented),
        (0x1c, _) => Ok(Operation::NotImplemented),
        (0x1d, _) => Ok(Operation::Syscall(opcode)),
        (0x20, _) => Ok(Operation::Syscall(opcode)),
        (0x21, _) => Ok(Operation::NotImplemented),
        (0x30, _) => Ok(Operation::Syscall(opcode)),
        (0x31, _) => Ok(Operation::Syscall(opcode)),
        (0x32, _) => Ok(Operation::Syscall(opcode)),
        (0x33, _) => Ok(Operation::Syscall(opcode)),
        (0x34, _) => Ok(Operation::Syscall(opcode)),
        (0x35, _) => Ok(Operation::Syscall(opcode)),
        (0x36, _) => Ok(Operation::Syscall(opcode)),
        (0x37, _) => Ok(Operation::Syscall(opcode)),
        (0x38, _) => Ok(Operation::Syscall(opcode)),
        (0x39, _) => Ok(Operation::Syscall(opcode)),
        (0x3a, _) => Ok(Operation::Syscall(opcode)),
        (0x3b, _) => Ok(Operation::Syscall(opcode)),
        (0x3c, _) => Ok(Operation::Syscall(opcode)),
        (0x3d, _) => Ok(Operation::Syscall(opcode)),
        (0x3e, _) => Ok(Operation::Syscall(opcode)),
        (0x3f, _) => Ok(Operation::Syscall(opcode)),
        (0x40, _) => Ok(Operation::Syscall(opcode)),
        (0x41, _) => Ok(Operation::Syscall(opcode)),
        (0x42, _) => Ok(Operation::Syscall(opcode)),
        (0x43, _) => Ok(Operation::Syscall(opcode)),
        (0x44, _) => Ok(Operation::Syscall(opcode)),
        (0x45, _) => Ok(Operation::Syscall(opcode)),
        (0x46, _) => Ok(Operation::Syscall(opcode)),
        (0x47, _) => Ok(Operation::Syscall(opcode)),
        (0x48, _) => Ok(Operation::Syscall(opcode)),
        (0x49, _) => Ok(Operation::NotImplemented),
        (0x50, _) => Ok(Operation::NotImplemented),
        (0x51, _) => Ok(Operation::Syscall(opcode)),
        (0x52, _) => Ok(Operation::Syscall(opcode)),
        (0x53, _) => Ok(Operation::Syscall(opcode)),
        (0x54, _) => Ok(Operation::Syscall(opcode)),
        (0x55, _) => Ok(Operation::Syscall(opcode)),
        (0x56, _) => Ok(Operation::NotImplemented),
        (0x57, _) => Ok(Operation::NotImplemented),
        (0x58, _) => Ok(Operation::NotImplemented),
        (0x59, _) => Ok(Operation::Syscall(opcode)),
        (0x5a, _) => Ok(Operation::NotImplemented),
        (0x5b, _) => Ok(Operation::NotImplemented),
        (0x5c, true) => Ok(Operation::NotImplemented),
        (0x5d, true) => Ok(Operation::NotImplemented),
        (0x5e, true) => Ok(Operation::NotImplemented),
        (0x5f, true) => Ok(Operation::NotImplemented),
        (0x60..=0x7f, _) => Ok(Operation::Push(opcode & 0x1f)),
        (0x80..=0x8f, _) => Ok(Operation::Dup(opcode & 0xf)),
        (0x90..=0x9f, _) => Ok(Operation::Swap(opcode & 0xf)),
        (0xa0, _) => Ok(Operation::Syscall(opcode)),
        (0xa1, _) => Ok(Operation::Syscall(opcode)),
        (0xa2, _) => Ok(Operation::Syscall(opcode)),
        (0xa3, _) => Ok(Operation::Syscall(opcode)),
        (0xa4, _) => Ok(Operation::Syscall(opcode)),
        (0xf0, _) => Ok(Operation::Syscall(opcode)),
        (0xf1, _) => Ok(Operation::Syscall(opcode)),
        (0xf2, _) => Ok(Operation::Syscall(opcode)),
        (0xf3, _) => Ok(Operation::Syscall(opcode)),
        (0xf4, _) => Ok(Operation::Syscall(opcode)),
        (0xf5, _) => Ok(Operation::Syscall(opcode)),
        (0xf6, true) => Ok(Operation::NotImplemented),
        (0xf7, true) => Ok(Operation::NotImplemented),
        (0xf8, true) => Ok(Operation::NotImplemented),
        (0xf9, true) => Ok(Operation::ExitKernel),
        (0xfa, _) => Ok(Operation::Syscall(opcode)),
        (0xfb, true) => Ok(Operation::NotImplemented),
        (0xfc, true) => Ok(Operation::NotImplemented),
        (0xfd, _) => Ok(Operation::Syscall(opcode)),
        (0xff, _) => Ok(Operation::Syscall(opcode)),
        _ => Err(ProgramError::InvalidOpcode),
    }
}

fn fill_op_flag<F: Field>(op: Operation, row: &mut CpuColumnsView<F>) {
    let flags = &mut row.op;
    *match op {
        Operation::Push(_) => &mut flags.push,
        Operation::Dup(_) => &mut flags.dup,
        Operation::Swap(_) => &mut flags.swap,
        Operation::Iszero => &mut flags.iszero,
        Operation::Not => &mut flags.not,
        Operation::Syscall(_) => &mut flags.syscall,
        Operation::Eq => &mut flags.eq,
        Operation::ExitKernel => &mut flags.exit_kernel,
        Operation::BinaryLogic(logic::Op::And) => &mut flags.and,
        Operation::BinaryLogic(logic::Op::Or) => &mut flags.or,
        Operation::BinaryLogic(logic::Op::Xor) => &mut flags.xor,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Add) => &mut flags.add,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Mul) => &mut flags.mul,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Sub) => &mut flags.sub,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Div) => &mut flags.div,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Mod) => &mut flags.mod_,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Lt) => &mut flags.lt,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Gt) => &mut flags.gt,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Shl) => &mut flags.shl,
        Operation::BinaryArithmetic(arithmetic::BinaryOperator::Shr) => &mut flags.shr,
        Operation::TernaryArithmetic(arithmetic::TernaryOperator::AddMod) => &mut flags.addmod,
        Operation::TernaryArithmetic(arithmetic::TernaryOperator::MulMod) => &mut flags.mulmod,
        _ => panic!("operation not implemented: {:?}", op),
    } = F::ONE;
}

fn perform_op<F: Field>(
    op: Operation,
    registers_state: RegistersState,
    memory_state: &MemoryState,
    traces: &mut Traces<F>,
    row: CpuColumnsView<F>,
) -> Result<RegistersState, ProgramError> {
    let mut new_registers_state = match op {
        Operation::Push(n) => generate_push(n, registers_state, memory_state, traces, row)?,
        Operation::Dup(n) => generate_dup(n, registers_state, memory_state, traces, row)?,
        Operation::Swap(n) => generate_swap(n, registers_state, memory_state, traces, row)?,
        Operation::Iszero => generate_iszero(registers_state, memory_state, traces, row)?,
        Operation::Not => generate_not(registers_state, memory_state, traces, row)?,
        Operation::Syscall(opcode) => {
            generate_syscall(opcode, registers_state, memory_state, traces, row)?
        }
        Operation::Eq => generate_eq(registers_state, memory_state, traces, row)?,
        Operation::ExitKernel => generate_exit_kernel(registers_state, memory_state, traces, row)?,
        Operation::BinaryLogic(binary_logic_op) => {
            generate_binary_logic_op(binary_logic_op, registers_state, memory_state, traces, row)?
        }
        _ => panic!("operation not implemented: {:?}", op),
    };

    new_registers_state.program_counter += match op {
        Operation::Syscall(_) | Operation::ExitKernel => 0,
        Operation::Push(n) => n as usize + 2,
        _ => 1,
    };

    Ok(new_registers_state)
}

fn try_perform_instruction<F: Field>(
    registers_state: RegistersState,
    memory_state: &MemoryState,
    traces: &mut Traces<F>,
) -> Result<RegistersState, ProgramError> {
    let mut row: CpuColumnsView<F> = CpuColumnsView::default();
    row.is_cpu_cycle = F::ONE;

    let opcode = read_code_memory(registers_state, memory_state, traces, &mut row);
    let op = decode(registers_state, opcode)?;
    log::trace!(
        "Executing {}={:?} at {}",
        opcode,
        op,
        registers_state.program_counter
    );
    // TODO: Temporarily slowing down so we can view logs easily.
    std::thread::sleep(std::time::Duration::from_millis(200));
    fill_op_flag(op, &mut row);

    perform_op(op, registers_state, memory_state, traces, row)
}

fn handle_error<F: Field>(
    _registers_state: RegistersState,
    _memory_state: &MemoryState,
    _traces: &mut Traces<F>,
) -> RegistersState {
    todo!("constraints for exception handling are not implemented");
}

pub(crate) fn transition<F: Field>(
    registers_state: RegistersState,
    memory_state: &MemoryState,
    traces: &mut Traces<F>,
) -> RegistersState {
    let checkpoint = traces.checkpoint();
    let result = try_perform_instruction(registers_state, memory_state, traces);

    match result {
        Ok(new_registers_state) => new_registers_state,
        Err(_) => {
            traces.rollback(checkpoint);
            if registers_state.is_kernel {
                panic!("exception in kernel mode");
            }
            handle_error(registers_state, memory_state, traces)
        }
    }
}
