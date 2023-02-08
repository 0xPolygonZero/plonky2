use crate::witness::operation::Operation;

const KERNEL_ONLY_INSTR: u64 = 0;
const G_JUMPDEST: u64 = 1;
const G_BASE: u64 = 2;
const G_VERYLOW: u64 = 3;
const G_LOW: u64 = 5;
const G_MID: u64 = 8;
const G_HIGH: u64 = 10;

pub(crate) fn gas_to_charge(op: Operation) -> u64 {
    use crate::arithmetic::BinaryOperator::*;
    use crate::witness::operation::Operation::*;
    match op {
        Iszero => G_VERYLOW,
        Not => G_VERYLOW,
        Byte => G_VERYLOW,
        Syscall(_) => KERNEL_ONLY_INSTR,
        Eq => G_VERYLOW,
        BinaryLogic(_) => G_VERYLOW,
        BinaryArithmetic(Add) => G_VERYLOW,
        BinaryArithmetic(Mul) => G_LOW,
        BinaryArithmetic(Sub) => G_VERYLOW,
        BinaryArithmetic(Div) => G_LOW,
        BinaryArithmetic(Mod) => G_LOW,
        BinaryArithmetic(Lt) => G_VERYLOW,
        BinaryArithmetic(Gt) => G_VERYLOW,
        BinaryArithmetic(Shl) => G_VERYLOW,
        BinaryArithmetic(Shr) => G_VERYLOW,
        BinaryArithmetic(AddFp254) => KERNEL_ONLY_INSTR,
        BinaryArithmetic(MulFp254) => KERNEL_ONLY_INSTR,
        BinaryArithmetic(SubFp254) => KERNEL_ONLY_INSTR,
        TernaryArithmetic(_) => G_MID,
        KeccakGeneral => KERNEL_ONLY_INSTR,
        ProverInput => KERNEL_ONLY_INSTR,
        Pop => G_BASE,
        Jump => G_MID,
        Jumpi => G_HIGH,
        Pc => G_BASE,
        Jumpdest => G_JUMPDEST,
        Push(_) => G_VERYLOW,
        Dup(_) => G_VERYLOW,
        Swap(_) => G_VERYLOW,
        GetContext => KERNEL_ONLY_INSTR,
        SetContext => KERNEL_ONLY_INSTR,
        ExitKernel => KERNEL_ONLY_INSTR,
        MloadGeneral => KERNEL_ONLY_INSTR,
        MstoreGeneral => KERNEL_ONLY_INSTR,
    }
}
