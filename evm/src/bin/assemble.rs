use std::env;
use std::fs;

use hex::encode;
use plonky2_evm::cpu::kernel::assemble_to_bytes;

fn main() {
    let mut args = env::args();
    args.next();
    let file_contents: Vec<_> = args.map(|path| fs::read_to_string(path).unwrap()).collect();
    let assembled = assemble_to_bytes(&file_contents[..]);
    println!("{}", encode(assembled));
}
