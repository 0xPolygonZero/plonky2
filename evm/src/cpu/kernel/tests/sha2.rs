use std::str::FromStr;

use anyhow::Result;
use ascii::AsciiStr;
use ethereum_types::U256;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run;
use crate::memory::segments::Segment;

#[test]
fn test_sha2() -> Result<()> {
    let kernel = combined_kernel();
    let sha2 = kernel.global_labels["sha2"];

    let mut rng = thread_rng();
    let num_bytes = rng.gen_range(1..17);
    let mut bytes: Vec<U256> = Vec::with_capacity(num_bytes);
    for _ in 0..num_bytes {
        let byte: u8 = rng.gen();
        let mut v = vec![0; 31];
        v.push(byte);
        let v2: [u8; 32] = v.try_into().unwrap();
        bytes.push(U256::from(v2));
    }

    dbg!(num_bytes);
    dbg!(bytes.clone());

    let message = "blargh blargh blargh blarh blargh blargh blargh blarghooo";
    let num_bytes = message.len();
    dbg!(num_bytes);

    let mut hasher = Sha256::new();
    hasher.update(message);
    let expected = format!("{:02X}", hasher.finalize());

    let bytes: Vec<U256> = AsciiStr::from_ascii(message)
        .unwrap()
        .as_bytes()
        .iter()
        .map(|&x| U256::from(x as u32))
        .collect();

    let mut store_initial_stack = vec![U256::from(num_bytes)];
    store_initial_stack.extend(bytes);
    store_initial_stack.push(U256::from_str("0xdeadbeef").unwrap());
    store_initial_stack.reverse();

    let after_sha2 = run(
        &kernel.code,
        sha2,
        store_initial_stack,
        &kernel.prover_inputs,
    )?;

    let stack_after_storing = after_sha2.stack();

    dbg!(stack_after_storing.clone());

    let result = stack_after_storing.clone()[1];
    let actual = format!("{:02X}", result);
    dbg!(expected);
    dbg!(actual);

    // assert_eq!(expected, actual);

    let memory_after_storing = after_sha2.memory;
    let mem = memory_after_storing.context_memory[0].segments[Segment::KernelGeneral as usize]
        .content
        .clone();
    // dbg!(&mem[0..65]);

    let num_blocks = (num_bytes+8)/64 + 1;
    let message_schedule_start = 64 * num_blocks + 2;
    // dbg!(&mem[message_schedule_start..message_schedule_start+256]);
    // dbg!(&mem[message_schedule_start+256..message_schedule_start+512]);

    Ok(())
}
