use std::collections::HashMap;

use super::ast::PushTarget;
use crate::cpu::kernel::{
    ast::{Function, Item},
    opcodes::{opcode_ordinal, push_ordinal},
};

/// The number of bytes to push when pushing an offset within the code (i.e. when assembling jumps).
/// Ideally we would automatically use the minimal number of bytes required, but that would be
/// nontrivial given the circular dependency between an offset and its size.
const BYTES_PER_OFFSET: usize = 3;

#[derive(PartialEq, Eq, Debug)]
#[allow(dead_code)] // TODO: Should be used once witness generation is done.
pub struct Kernel {
    code: Vec<u8>,
    function_offsets: HashMap<String, usize>,
}

pub(crate) fn assemble(functions: Vec<Function>) -> Kernel {
    let mut code = vec![];
    let mut function_offsets = HashMap::new();
    for function in functions {
        function_offsets.insert(function.name, code.len());
        assemble_function(function.body, &mut code);
    }
    Kernel {
        code,
        function_offsets,
    }
}

fn assemble_function(body: Vec<Item>, code: &mut Vec<u8>) {
    // First discover the offset of each label  in this function.
    let mut label_offsets = HashMap::<String, usize>::new();
    let mut offset = code.len();
    for item in &body {
        match item {
            Item::LabelDeclaration(label) => {
                label_offsets.insert(label.clone(), offset);
            }
            Item::Push(target) => offset += 1 + push_target_size(target),
            Item::StandardOp(_) => offset += 1,
            Item::Literal(hex) => offset += hex.to_bytes().len(),
        }
    }

    // Now that we have label offsets, we can assemble the function.
    for item in body {
        match item {
            Item::LabelDeclaration(_) => {
                // Nothing to do; we processed labels in the prior phase.
            }
            Item::Push(target) => {
                let target_bytes: Vec<u8> = match target {
                    PushTarget::Literal(literal) => literal.to_trimmed_be_bytes(),
                    PushTarget::Label(label) => {
                        let offset = label_offsets[&label];
                        // We want the BYTES_PER_OFFSET least significant bytes in BE order.
                        // It's easiest to rev the first BYTES_PER_OFFSET bytes of the LE encoding.
                        (0..BYTES_PER_OFFSET)
                            .rev()
                            .map(|i| offset.to_le_bytes()[i])
                            .collect()
                    }
                };
                code.push(push_ordinal(target_bytes.len()));
                code.extend(target_bytes);
            }
            Item::StandardOp(opcode) => {
                code.push(opcode_ordinal(&opcode));
            }
            Item::Literal(hex) => code.extend(hex.to_bytes()),
        }
    }

    assert_eq!(
        code.len(),
        offset,
        "The two phases gave different code lengths"
    );
}

/// The size of a `PushTarget`, in bytes.
fn push_target_size(target: &PushTarget) -> usize {
    match target {
        PushTarget::Literal(lit) => lit.to_trimmed_be_bytes().len(),
        PushTarget::Label(_) => BYTES_PER_OFFSET,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::cpu::kernel::{assembler::*, ast::*};

    #[test]
    fn two_functions() {
        // We will test two simple functions, with a label and a jump, to ensure that jump offsets
        // are correctly shifted based on the offset of the containing function.

        let function_1 = Function {
            name: "function_1".to_string(),
            body: vec![
                Item::StandardOp("ADD".to_string()),
                Item::StandardOp("MUL".to_string()),
            ],
        };

        let function_2 = Function {
            name: "function_2".to_string(),
            body: vec![
                Item::StandardOp("DIV".to_string()),
                Item::LabelDeclaration("mylabel".to_string()),
                Item::StandardOp("JUMPDEST".to_string()),
                Item::StandardOp("MOD".to_string()),
                Item::Push(PushTarget::Label("mylabel".to_string())),
                Item::StandardOp("JUMP".to_string()),
            ],
        };

        let expected_code = vec![
            opcode_ordinal("ADD"),
            opcode_ordinal("MUL"),
            opcode_ordinal("DIV"),
            opcode_ordinal("JUMPDEST"),
            opcode_ordinal("MOD"),
            push_ordinal(BYTES_PER_OFFSET),
            // The label offset, 3, in 3-byte BE form.
            0,
            0,
            3,
            opcode_ordinal("JUMP"),
        ];

        let mut expected_function_offsets = HashMap::new();
        expected_function_offsets.insert("function_1".to_string(), 0);
        expected_function_offsets.insert("function_2".to_string(), 2);

        let expected_kernel = Kernel {
            code: expected_code,
            function_offsets: expected_function_offsets,
        };

        let program = vec![function_1, function_2];
        assert_eq!(assemble(program), expected_kernel);
    }
}
