#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegistersState {
    pub program_counter: u32,
    pub is_kernel: bool,
    pub stack_len: u32,
    pub context: u32,
}
