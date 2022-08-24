use std::str::FromStr;

use anyhow::Result;
use ascii::AsciiStr;
use ethereum_types::U256;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run;

#[test]
fn test_sha2() -> Result<()> {
    let kernel = combined_kernel();
    let sha2 = kernel.global_labels["sha2"];

    let mut rng = thread_rng();

    let num_bytes = rng.gen_range(1..10000);
    let message: String = rng
        .sample_iter(&Alphanumeric)
        .take(num_bytes)
        .map(char::from)
        .collect();
    dbg!(num_bytes);

    let mut hasher = Sha256::new();
    hasher.update(message.clone());
    let expected = format!("{:02X}", hasher.finalize());

    let bytes: Vec<U256> = AsciiStr::from_ascii(&message)
        .unwrap()
        .as_bytes()
        .iter()
        .map(|&x| U256::from(x as u32))
        .collect();

    let mut initial_stack = vec![U256::from(num_bytes)];
    initial_stack.extend(bytes);
    initial_stack.push(U256::from_str("0xdeadbeef").unwrap());
    initial_stack.reverse();

    let after_sha2 = run(&kernel.code, sha2, initial_stack, &kernel.prover_inputs)?;

    let stack_after_sha2 = after_sha2.stack();

    let result = stack_after_sha2.clone()[1];
    let actual = format!("{:02X}", result);
    dbg!(expected);
    dbg!(actual);

    Ok(())
}
