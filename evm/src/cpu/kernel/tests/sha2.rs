use core::num;
use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run;

#[test]
fn test_sha2_store() -> Result<()> {
    let kernel = combined_kernel();
    let sha2_store = kernel.global_labels["sha2_store"];
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

    let mut initial_stack = vec![U256::from(num_bytes)];
    initial_stack.extend(bytes);
    initial_stack.push(U256::from_str("0xdeadbeef").unwrap());
    dbg!(initial_stack.clone());
    let stack_with_kernel = run(&kernel.code, sha2_store, initial_stack)?.stack;
    dbg!(stack_with_kernel);

    // let expected_stack = todo!();
    // assert_eq!(stack_with_kernel, expected_stack);

    Ok(())
}

/*#[test]
fn test_sha2() -> Result<()> {
    let kernel = combined_kernel();
    let sha2_store = kernel.global_labels["sha2_store"];
    let sha2_pad = kernel.global_labels["sha2_pad"];
    let mut rng = thread_rng();
    let a = U256([0; 4].map(|_| rng.gen()));
    let b = U256([0; 4].map(|_| rng.gen()));

    let initial_stack = vec![U256::from_str("0xdeadbeef")?, b, a];
    let stack_with_kernel = run(&kernel.code, exp, initial_stack)?.stack;
    let initial_stack = vec![b, a];
    let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP

    let expected_stack = todo!();
    assert_eq!(stack_with_kernel, expected_stack);

    Ok(())
}*/
