pub mod aggregator;
pub mod assembler;
mod ast;
pub(crate) mod keccak_util;
mod opcodes;
mod parser;
mod stack_manipulation;

#[cfg(test)]
mod interpreter;
#[cfg(test)]
mod tests;

use assembler::assemble;
use parser::parse;

use crate::cpu::kernel::aggregator::evm_constants;

/// Assemble files, outputting bytes.
/// This is for debugging the kernel only.
pub fn assemble_to_bytes(files: &[String]) -> Vec<u8> {
    let parsed_files: Vec<_> = files.iter().map(|f| parse(f)).collect();
    let kernel = assemble(parsed_files, evm_constants());
    kernel.code
}
