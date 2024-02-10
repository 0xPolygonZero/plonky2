use std::collections::HashSet;

use anyhow::Result;
use ethereum_types::{Address, U256};
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata::{
    AccessedAddressesLen, AccessedStorageKeysLen,
};
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment::{AccessedAddresses, AccessedStorageKeys};
use crate::witness::memory::MemoryAddress;

#[test]
fn test_insert_accessed_addresses() -> Result<()> {
    let insert_accessed_addresses = KERNEL.global_labels["insert_accessed_addresses"];

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
    let initial_stack = vec![retaddr, U256::from(addr_in_list.0.as_slice())];
    let mut interpreter = Interpreter::new_with_kernel(insert_accessed_addresses, initial_stack);
    for i in 0..n {
        let addr = U256::from(addresses[i].0.as_slice());
        interpreter
            .generation_state
            .memory
            .set(MemoryAddress::new(0, AccessedAddresses, i), addr);
    }
    interpreter.generation_state.memory.set(
        MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap(),
        U256::from(n),
    );
    interpreter.run()?;
    assert_eq!(interpreter.stack(), &[U256::zero()]);
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap()),
        U256::from(n)
    );

    // Test for address not in list.
    let initial_stack = vec![retaddr, U256::from(addr_not_in_list.0.as_slice())];
    let mut interpreter = Interpreter::new_with_kernel(insert_accessed_addresses, initial_stack);
    for i in 0..n {
        let addr = U256::from(addresses[i].0.as_slice());
        interpreter
            .generation_state
            .memory
            .set(MemoryAddress::new(0, AccessedAddresses, i), addr);
    }
    interpreter.generation_state.memory.set(
        MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap(),
        U256::from(n),
    );
    interpreter.run()?;
    assert_eq!(interpreter.stack(), &[U256::one()]);
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap()),
        U256::from(n + 1)
    );
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new(0, AccessedAddresses, n)),
        U256::from(addr_not_in_list.0.as_slice())
    );

    Ok(())
}

#[test]
fn test_insert_accessed_storage_keys() -> Result<()> {
    let insert_accessed_storage_keys = KERNEL.global_labels["insert_accessed_storage_keys"];

    let retaddr = 0xdeadbeefu32.into();
    let mut rng = thread_rng();
    let n = rng.gen_range(1..10);
    let storage_keys = (0..n)
        .map(|_| (rng.gen::<Address>(), U256(rng.gen()), U256(rng.gen())))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<(Address, U256, U256)>>();
    let storage_key_in_list = storage_keys[rng.gen_range(0..n)];
    let storage_key_not_in_list = (rng.gen::<Address>(), U256(rng.gen()), U256(rng.gen()));
    assert!(
        !storage_keys.contains(&storage_key_not_in_list),
        "Cosmic luck or bad RNG?"
    );

    // Test for storage key already in list.
    let initial_stack = vec![
        retaddr,
        storage_key_in_list.2,
        storage_key_in_list.1,
        U256::from(storage_key_in_list.0 .0.as_slice()),
    ];
    let mut interpreter = Interpreter::new_with_kernel(insert_accessed_storage_keys, initial_stack);
    for i in 0..n {
        let addr = U256::from(storage_keys[i].0 .0.as_slice());
        interpreter
            .generation_state
            .memory
            .set(MemoryAddress::new(0, AccessedStorageKeys, 3 * i), addr);
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 3 * i + 1),
            storage_keys[i].1,
        );
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 3 * i + 2),
            storage_keys[i].2,
        );
    }
    interpreter.generation_state.memory.set(
        MemoryAddress::new_bundle(U256::from(AccessedStorageKeysLen as usize)).unwrap(),
        U256::from(3 * n),
    );
    interpreter.run()?;
    assert_eq!(interpreter.stack(), &[storage_key_in_list.2, U256::zero()]);
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new_bundle(U256::from(AccessedStorageKeysLen as usize)).unwrap()),
        U256::from(3 * n)
    );

    // Test for storage key not in list.
    let initial_stack = vec![
        retaddr,
        storage_key_not_in_list.2,
        storage_key_not_in_list.1,
        U256::from(storage_key_not_in_list.0 .0.as_slice()),
    ];
    let mut interpreter = Interpreter::new_with_kernel(insert_accessed_storage_keys, initial_stack);
    for i in 0..n {
        let addr = U256::from(storage_keys[i].0 .0.as_slice());
        interpreter
            .generation_state
            .memory
            .set(MemoryAddress::new(0, AccessedStorageKeys, 3 * i), addr);
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 3 * i + 1),
            storage_keys[i].1,
        );
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 3 * i + 2),
            storage_keys[i].2,
        );
    }
    interpreter.generation_state.memory.set(
        MemoryAddress::new_bundle(U256::from(AccessedStorageKeysLen as usize)).unwrap(),
        U256::from(3 * n),
    );
    interpreter.run()?;
    assert_eq!(
        interpreter.stack(),
        &[storage_key_not_in_list.2, U256::one()]
    );
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new_bundle(U256::from(AccessedStorageKeysLen as usize)).unwrap()),
        U256::from(3 * (n + 1))
    );
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new(0, AccessedStorageKeys, 3 * n,)),
        U256::from(storage_key_not_in_list.0 .0.as_slice())
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedStorageKeys,
            3 * n + 1,
        )),
        storage_key_not_in_list.1
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedStorageKeys,
            3 * n + 2,
        )),
        storage_key_not_in_list.2
    );

    Ok(())
}
