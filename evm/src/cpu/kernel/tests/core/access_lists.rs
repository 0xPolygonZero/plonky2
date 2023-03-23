use std::collections::HashSet;

use anyhow::Result;
use ethereum_types::{Address, U256};
use rand::{thread_rng, Rng};

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, InterpreterMemoryInitialization,
};
use crate::memory::segments::Segment::{AccessedAddresses, AccessedStorageKeys};
use crate::witness::memory::MemoryAddress;

#[test]
fn test_insert_accessed_addresses() -> Result<()> {
    let retaddr = 0xdeadbeefu32.into();
    let mut rng = thread_rng();
    let n = rng.gen_range(1..10);
    let addresses = (0..n)
        .map(|_| rng.gen::<Address>())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<Address>>();
    let addr_in_list = addresses[rng.gen_range(0..n)];
    let addr_not_in_list = rng.gen::<Address>();
    assert!(
        !addresses.contains(&addr_not_in_list),
        "Cosmic luck or bad RNG?"
    );

    // Test for address already in list.
    let initial_stack = vec![U256::from(addr_in_list.0.as_slice()), retaddr];
    let memory = {
        let mut mem = vec![];
        mem.push((0, vec![U256::from(n)]));
        for i in 0..n {
            mem.push((i + 1, vec![U256::from(addresses[i].0.as_slice())]));
        }
        mem
    };
    let mut interpreter_setup = InterpreterMemoryInitialization {
        label: "insert_accessed_addresses".to_string(),
        stack: initial_stack,
        segment: AccessedAddresses,
        memory,
    };
    let interpreter = run_interpreter_with_memory(interpreter_setup.clone())?;
    assert_eq!(interpreter.stack(), &[U256::zero()]);

    // Test for address not in list.
    let initial_stack = vec![U256::from(addr_not_in_list.0.as_slice()), retaddr];
    interpreter_setup.stack = initial_stack;
    let interpreter = run_interpreter_with_memory(interpreter_setup)?;
    assert_eq!(interpreter.stack(), &[U256::one()]);
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new(0, AccessedAddresses, 0)),
        U256::from(n + 1)
    );
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new(0, AccessedAddresses, n + 1)),
        U256::from(addr_not_in_list.0.as_slice())
    );

    Ok(())
}

#[test]
fn test_insert_accessed_storage_keys() -> Result<()> {
    let retaddr = 0xdeadbeefu32.into();
    let mut rng = thread_rng();
    let n = rng.gen_range(1..10);
    let storage_keys = (0..n)
        .map(|_| (rng.gen::<Address>(), U256(rng.gen())))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<(Address, U256)>>();
    let storage_key_in_list = storage_keys[rng.gen_range(0..n)];
    let storage_key_not_in_list = (rng.gen::<Address>(), U256(rng.gen()));
    assert!(
        !storage_keys.contains(&storage_key_not_in_list),
        "Cosmic luck or bad RNG?"
    );

    // Test for storage key already in list.
    let initial_stack = vec![
        U256::from(storage_key_in_list.0 .0.as_slice()),
        storage_key_in_list.1,
        retaddr,
    ];
    let memory = {
        let mut mem = vec![];
        mem.push((0, vec![U256::from(2 * n)]));
        for i in 0..n {
            mem.push((2 * i + 1, vec![U256::from(storage_keys[i].0 .0.as_slice())]));
            mem.push((2 * i + 2, vec![storage_keys[i].1]));
        }
        mem
    };
    let mut interpreter_setup = InterpreterMemoryInitialization {
        label: "insert_accessed_storage_keys".to_string(),
        stack: initial_stack,
        segment: AccessedStorageKeys,
        memory,
    };
    let interpreter = run_interpreter_with_memory(interpreter_setup.clone())?;
    assert_eq!(interpreter.stack(), &[U256::zero()]);

    // Test for storage key not in list.
    let initial_stack = vec![
        U256::from(storage_key_not_in_list.0 .0.as_slice()),
        storage_key_not_in_list.1,
        retaddr,
    ];
    interpreter_setup.stack = initial_stack;
    let interpreter = run_interpreter_with_memory(interpreter_setup)?;
    assert_eq!(interpreter.stack(), &[U256::one()]);
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new(0, AccessedStorageKeys, 0)),
        U256::from(2 * (n + 1))
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedStorageKeys,
            2 * n + 1
        )),
        U256::from(storage_key_not_in_list.0 .0.as_slice())
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedStorageKeys,
            2 * n + 2
        )),
        storage_key_not_in_list.1
    );

    Ok(())
}
