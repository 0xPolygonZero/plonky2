use std::collections::HashSet;

use anyhow::Result;
use ethereum_types::{Address, H160, U256};
use hashbrown::hash_map::rayon::IntoParIter;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata::{
    AccessedAddressesLen, AccessedStorageKeysLen,
};
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment::{self, AccessedAddresses, AccessedStorageKeys};
use crate::memory::segments::SEGMENT_SCALING_FACTOR;
use crate::witness::memory::MemoryAddress;

#[test]
fn test_init_access_lists() -> Result<()> {
    let init_label = KERNEL.global_labels["init_access_lists"];

    // Test for address already in list.
    let initial_stack = vec![0xdeadbeefu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(init_label, initial_stack);
    interpreter.run()?;

    assert!(interpreter.stack().is_empty());

    let acc_addr_list: Vec<U256> = (0..2)
        .map(|i| {
            interpreter.generation_state.memory.get(MemoryAddress::new(
                0,
                Segment::AccessedAddresses,
                i,
            ))
        })
        .collect();
    assert_eq!(
        vec![U256::MAX, (Segment::AccessedAddresses as usize).into(),],
        acc_addr_list
    );

    let acc_storage_keys: Vec<U256> = (0..4)
        .map(|i| {
            interpreter.generation_state.memory.get(MemoryAddress::new(
                0,
                Segment::AccessedStorageKeys,
                i,
            ))
        })
        .collect();

    assert_eq!(
        vec![
            U256::MAX,
            U256::zero(),
            U256::zero(),
            (Segment::AccessedStorageKeys as usize).into()
        ],
        acc_storage_keys
    );

    // test the list iteratior
    let mut list = interpreter
        .generation_state
        .get_addresses_access_list()
        .expect("Couldn't retrieve access list");

    let Some((pos_0, next_val_0)) = list.next() else {
        return Err(anyhow::Error::msg("Couldn't get value"));
    };
    assert_eq!(pos_0, 0);
    assert_eq!(next_val_0, U256::MAX);
    let Some((pos_0, next_val_0)) = list.next() else {
        return Err(anyhow::Error::msg("Couldn't get value"));
    };
    assert_eq!(pos_0, 0);
    Ok(())
}

#[test]
fn test_insert_address() -> Result<()> {
    let init_label = KERNEL.global_labels["init_access_lists"];

    // Test for address already in list.
    let initial_stack = vec![0xdeadbeefu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(init_label, initial_stack);
    interpreter.run()?;

    let insert_accessed_addresses = KERNEL.global_labels["insert_accessed_addresses"];

    let retaddr = 0xdeadbeefu32.into();
    let mut rng = thread_rng();
    let mut address: H160 = rng.gen();

    assert!(address != H160::zero(), "Cosmic luck or bad RNG?");

    interpreter.push(retaddr);
    interpreter.push(U256::from(address.0.as_slice()));
    interpreter.generation_state.registers.program_counter = insert_accessed_addresses;

    interpreter.run()?;
    assert_eq!(interpreter.stack(), &[U256::one()]);
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap()),
        U256::from(Segment::AccessedAddresses as usize + 4)
    );

    Ok(())
}

#[test]
fn test_insert_accessed_addresses() -> Result<()> {
    let init_access_lists = KERNEL.global_labels["init_access_lists"];

    // Test for address already in list.
    let initial_stack = vec![0xdeadbeefu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(init_access_lists, initial_stack);
    interpreter.run()?;

    let insert_accessed_addresses = KERNEL.global_labels["insert_accessed_addresses"];

    let retaddr = 0xdeadbeefu32.into();
    let mut rng = thread_rng();
    let n = 10;
    let mut addresses = (0..n)
        .map(|_| rng.gen::<Address>())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<Address>>();
    // The addresses must be sorted.
    addresses.sort();
    let addr_not_in_list = rng.gen::<Address>();
    assert!(
        !addresses.contains(&addr_not_in_list),
        "Cosmic luck or bad RNG?"
    );

    let offset = Segment::AccessedAddresses as usize;
    for i in 0..n {
        let addr = U256::from(addresses[i].0.as_slice());
        interpreter
            .generation_state
            .memory
            .set(MemoryAddress::new(0, AccessedAddresses, 2 + 2 * i), addr);
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedAddresses, 2 + 2 * i + 1),
            (offset + 2 + 2 * (i + 1)).into(),
        );
    }
    // Set (U256::MAX)-> (addreses[0]) and (addresses[n-1]) -> (U256::MAX)
    interpreter.generation_state.memory.set(
        MemoryAddress::new(0, AccessedAddresses, 1),
        (offset + 2).into(), // the address of (addr[0])
    );
    interpreter.generation_state.memory.set(
        MemoryAddress::new(0, AccessedAddresses, 2 + 2 * (n - 1) + 1),
        offset.into(), // the address of (U256::MAX)
    );

    // Set the segment length
    interpreter.generation_state.memory.set(
        MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap(),
        (offset + 2 * (n + 1)).into(), // The length of the access list is scaled
    );

    interpreter.push(U256::zero()); // We need something to pop
    for i in 0..10 {
        // Test for address already in list.
        let addr_in_list = addresses[i];
        interpreter.pop();
        interpreter.push(retaddr);
        interpreter.push(U256::from(addr_in_list.0.as_slice()));
        interpreter.generation_state.registers.program_counter = insert_accessed_addresses;
        interpreter.run()?;
        assert_eq!(interpreter.stack(), &[U256::zero()]);
        assert_eq!(
            interpreter
                .generation_state
                .memory
                .get(MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap()),
            U256::from(offset + 2 * (n + 1))
        );
    }

    // Test for address not in list.
    interpreter.pop(); // pop the last top of the stack
    interpreter.push(retaddr);
    interpreter.push(U256::from(addr_not_in_list.0.as_slice()));
    interpreter.generation_state.registers.program_counter = insert_accessed_addresses;

    interpreter.run()?;
    assert_eq!(interpreter.stack(), &[U256::one()]);
    assert_eq!(
        interpreter
            .generation_state
            .memory
            .get(MemoryAddress::new_bundle(U256::from(AccessedAddressesLen as usize)).unwrap()),
        U256::from(offset + 2 * (n + 2))
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedAddresses,
            2 * (n + 1)
        )),
        U256::from(addr_not_in_list.0.as_slice())
    );

    Ok(())
}

#[test]
fn test_insert_accessed_storage_keys() -> Result<()> {
    let init_access_lists = KERNEL.global_labels["init_access_lists"];

    // Test for address already in list.
    let initial_stack = vec![0xdeadbeefu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(init_access_lists, initial_stack);
    interpreter.run()?;

    let insert_accessed_storage_keys = KERNEL.global_labels["insert_accessed_storage_keys"];

    let retaddr = 0xdeadbeefu32.into();
    let mut rng = thread_rng();
    let n = 10;
    let mut storage_keys = (0..n)
        .map(|_| (rng.gen::<Address>(), U256(rng.gen()), U256(rng.gen())))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<(Address, U256, U256)>>();
    // Storage keys must be sorted
    storage_keys.sort();
    let storage_key_in_list = storage_keys[rng.gen_range(0..n)];
    let storage_key_not_in_list = (rng.gen::<Address>(), U256(rng.gen()), U256(rng.gen()));
    assert!(
        !storage_keys.contains(&storage_key_not_in_list),
        "Cosmic luck or bad RNG?"
    );

    let offset = Segment::AccessedStorageKeys as usize;
    for i in 0..n {
        let addr = U256::from(storage_keys[i].0 .0.as_slice());
        let key = storage_keys[i].1;
        let value = storage_keys[i].2;
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 4 * (i + 1)),
            addr,
        );
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 4 * (i + 1) + 1),
            key,
        );
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 4 * (i + 1) + 2),
            value,
        );
        interpreter.generation_state.memory.set(
            MemoryAddress::new(0, AccessedStorageKeys, 4 * (i + 1) + 3),
            (offset + 4 * (i + 2)).into(),
        );
    }

    // Set (U256::MAX)-> (storage_keys[0]) and (storage_keys[n-1]) -> (U256::MAX)
    interpreter.generation_state.memory.set(
        MemoryAddress::new(0, AccessedStorageKeys, 3),
        (offset + 4).into(), // the address of (addr[0])
    );
    interpreter.generation_state.memory.set(
        MemoryAddress::new(0, AccessedStorageKeys, 4 * n + 3),
        offset.into(), // the address of (U256::MAX)
    );

    // Set the segment length
    interpreter.generation_state.memory.set(
        MemoryAddress::new_bundle(U256::from(AccessedStorageKeysLen as usize)).unwrap(),
        (offset + 4 * (n + 1)).into(), // The length of the access list is scaled
    );
    // We need something to pop
    interpreter.push(U256::zero());
    interpreter.push(U256::zero());
    for i in 0..10 {
        // Test for storage key already in list.
        let (addr, key, value) = storage_keys[i];
        interpreter.pop();
        interpreter.pop();
        interpreter.push(retaddr);
        interpreter.push(value);
        interpreter.push(key);
        interpreter.push(U256::from(addr.0.as_slice()));
        interpreter.generation_state.registers.program_counter = insert_accessed_storage_keys;
        interpreter.run()?;
        assert_eq!(interpreter.stack(), &[value, U256::zero()]);
        assert_eq!(
            interpreter.generation_state.memory.get(
                MemoryAddress::new_bundle(U256::from(AccessedStorageKeysLen as usize)).unwrap()
            ),
            U256::from(offset + 4 * (n + 1))
        );
    }

    // Test for storage key not in list.
    interpreter.pop(); // pop the last top of the stack
    interpreter.pop();
    interpreter.push(retaddr);
    interpreter.push(storage_key_not_in_list.2);
    interpreter.push(storage_key_not_in_list.1);
    interpreter.push(U256::from(storage_key_not_in_list.0 .0.as_slice()));
    interpreter.generation_state.registers.program_counter = insert_accessed_storage_keys;

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
        U256::from(offset + 4 * (n + 2))
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedStorageKeys,
            4 * (n + 1)
        )),
        U256::from(storage_key_not_in_list.0 .0.as_slice())
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedStorageKeys,
            4 * (n + 1) + 1
        )),
        storage_key_not_in_list.1
    );
    assert_eq!(
        interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            AccessedStorageKeys,
            4 * (n + 1) + 2
        )),
        storage_key_not_in_list.2
    );

    Ok(())
}
