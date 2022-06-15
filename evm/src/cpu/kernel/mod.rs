pub mod aggregator;
pub mod assembler;
mod ast;
mod opcodes;
mod parser;

use assembler::assemble;
use parser::parse;

/// Assemble files, outputting bytes.
/// This is for debugging the kernel only.
pub fn assemble_to_bytes(files: &[String]) -> Vec<u8> {
    let parsed_files: Vec<_> = files.iter().map(|f| parse(f)).collect();
    let kernel = assemble(parsed_files);
    kernel.code
}
