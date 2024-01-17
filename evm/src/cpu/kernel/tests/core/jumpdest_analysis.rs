use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::opcodes::{get_opcode, get_push_opcode};
use crate::witness::operation::CONTEXT_SCALING_FACTOR;

#[test]
fn test_jumpdest_analysis() -> Result<()> {
    let jumpdest_analysis = KERNEL.global_labels["jumpdest_analysis"];
    const CONTEXT: usize = 3; // arbitrary

    let add = get_opcode("ADD");
    let push2 = get_push_opcode(2);
    let jumpdest = get_opcode("JUMPDEST");

    #[rustfmt::skip]
    let code: Vec<u8> = vec![
        add,
        jumpdest,
        push2,
        jumpdest, // part of PUSH2
        jumpdest, // part of PUSH2
        jumpdest,
        add,
        jumpdest,
    ];

    let jumpdest_bits = vec![false, true, false, false, false, true, false, true];

    // Contract creation transaction.
    let initial_stack = vec![
        0xDEADBEEFu32.into(),
        code.len().into(),
        U256::from(CONTEXT) << CONTEXT_SCALING_FACTOR,
    ];
    let mut interpreter = Interpreter::new_with_kernel(jumpdest_analysis, initial_stack);
    interpreter.set_code(CONTEXT, code);
    interpreter.set_jumpdest_analysis_inputs(HashMap::from([(
        3,
        BTreeSet::from_iter(
            jumpdest_bits
                .iter()
                .enumerate()
                .filter(|&(_, &x)| x)
                .map(|(i, _)| i),
        ),
    )]));

    assert_eq!(
        interpreter.generation_state.jumpdest_table,
        Some(HashMap::from([(3, vec![0, 1, 0, 5, 0, 7])]))
    );

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    assert_eq!(jumpdest_bits, interpreter.get_jumpdest_bits(3));

    Ok(())
}

#[test]
fn test_packed_verification() -> Result<()> {
    let jumpdest_analysis = KERNEL.global_labels["jumpdest_analysis"];
    const CONTEXT: usize = 3; // arbitrary

    let add = get_opcode("ADD");
    let jumpdest = get_opcode("JUMPDEST");

    // The last push(i=0) is 0x5f which is not a valid opcode. However, this
    // is still meaningful for the test and makes things easier
    let mut code: Vec<u8> = std::iter::once(add)
        .chain(
            (0..=31)
                .rev()
                .map(get_push_opcode)
                .chain(std::iter::once(jumpdest)),
        )
        .collect();

    let jumpdest_bits: Vec<bool> = std::iter::repeat(false)
        .take(33)
        .chain(std::iter::once(true))
        .collect();

    // Contract creation transaction.
    let initial_stack = vec![
        0xDEADBEEFu32.into(),
        code.len().into(),
        U256::from(CONTEXT) << CONTEXT_SCALING_FACTOR,
    ];
    let mut interpreter = Interpreter::new_with_kernel(jumpdest_analysis, initial_stack.clone());
    interpreter.set_code(CONTEXT, code.clone());
    interpreter.generation_state.jumpdest_table = Some(HashMap::from([(3, vec![1, 33])]));

    interpreter.run()?;

    assert_eq!(jumpdest_bits, interpreter.get_jumpdest_bits(CONTEXT));

    // If we add 1 to each opcode the jumpdest at position 32 is never a valid jumpdest
    for i in 1..=32 {
        code[i] += 1;
        let mut interpreter =
            Interpreter::new_with_kernel(jumpdest_analysis, initial_stack.clone());
        interpreter.set_code(CONTEXT, code.clone());
        interpreter.generation_state.jumpdest_table = Some(HashMap::from([(3, vec![1, 33])]));

        interpreter.run()?;

        assert!(interpreter.get_jumpdest_bits(CONTEXT).is_empty());

        code[i] -= 1;
    }

    Ok(())
}
