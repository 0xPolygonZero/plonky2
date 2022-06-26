use std::collections::HashMap;

use super::ast::PushTarget;
use crate::cpu::kernel::{
    ast::{File, Item},
    opcodes::{get_opcode, get_push_opcode},
};

/// The number of bytes to push when pushing an offset within the code (i.e. when assembling jumps).
/// Ideally we would automatically use the minimal number of bytes required, but that would be
/// nontrivial given the circular dependency between an offset and its size.
const BYTES_PER_OFFSET: u8 = 3;

#[derive(PartialEq, Eq, Debug)]
pub struct Kernel {
    code: Vec<u8>,
    global_labels: HashMap<String, usize>,
}

pub(crate) fn assemble(files: Vec<File>) -> Kernel {
    let macros = find_macros(&files);
    let mut code = vec![];
    let mut global_labels = HashMap::new();
    for file in files {
        let expanded_file = expand_macros(file.body, &macros);
        assemble_file(expanded_file, &mut code, &mut global_labels);
    }
    Kernel {
        code,
        global_labels,
    }
}

fn find_macros(files: &[File]) -> HashMap<String, Vec<Item>> {
    let mut macros = HashMap::new();
    for file in files {
        for item in &file.body {
            if let Item::MacroDef(name, items) = item {
                macros.insert(name.clone(), items.clone());
            }
        }
    }
    macros
}

fn expand_macros(body: Vec<Item>, macros: &HashMap<String, Vec<Item>>) -> Vec<Item> {
    let mut expanded = vec![];
    for item in body {
        match item {
            Item::MacroDef(_, _) => {
                // At this phase, we no longer need macro definitions.
            }
            Item::MacroCall(m) => {
                let mut expanded_item = macros
                    .get(&m)
                    .cloned()
                    .unwrap_or_else(|| panic!("No such macro: {}", m));
                // Recursively expand any macros in the expanded code.
                expanded_item = expand_macros(expanded_item, macros);
                expanded.extend(expanded_item);
            }
            item => {
                expanded.push(item);
            }
        }
    }
    expanded
}

fn assemble_file(body: Vec<Item>, code: &mut Vec<u8>, global_labels: &mut HashMap<String, usize>) {
    // First discover the offset of each label in this file.
    let mut local_labels = HashMap::<String, usize>::new();
    let mut offset = code.len();
    for item in &body {
        match item {
            Item::MacroDef(_, _) | Item::MacroCall(_) => {
                panic!("Macros should have been expanded already")
            }
            Item::GlobalLabelDeclaration(label) => {
                let old = global_labels.insert(label.clone(), offset);
                assert!(old.is_none(), "Duplicate global label: {}", label);
            }
            Item::LocalLabelDeclaration(label) => {
                let old = local_labels.insert(label.clone(), offset);
                assert!(old.is_none(), "Duplicate local label: {}", label);
            }
            Item::Push(target) => offset += 1 + push_target_size(target) as usize,
            Item::StandardOp(_) => offset += 1,
            Item::Bytes(bytes) => offset += bytes.len(),
        }
    }

    // Now that we have label offsets, we can assemble the file.
    for item in body {
        match item {
            Item::MacroDef(_, _) | Item::MacroCall(_) => {
                panic!("Macros should have been expanded already")
            }
            Item::GlobalLabelDeclaration(_) | Item::LocalLabelDeclaration(_) => {
                // Nothing to do; we processed labels in the prior phase.
            }
            Item::Push(target) => {
                let target_bytes: Vec<u8> = match target {
                    PushTarget::Literal(literal) => literal.to_trimmed_be_bytes(),
                    PushTarget::Label(label) => {
                        let offset = local_labels
                            .get(&label)
                            .or_else(|| global_labels.get(&label))
                            .unwrap_or_else(|| panic!("No such label: {}", label));
                        // We want the BYTES_PER_OFFSET least significant bytes in BE order.
                        // It's easiest to rev the first BYTES_PER_OFFSET bytes of the LE encoding.
                        (0..BYTES_PER_OFFSET)
                            .rev()
                            .map(|i| offset.to_le_bytes()[i as usize])
                            .collect()
                    }
                };
                code.push(get_push_opcode(target_bytes.len() as u8));
                code.extend(target_bytes);
            }
            Item::StandardOp(opcode) => {
                code.push(get_opcode(&opcode));
            }
            Item::Bytes(bytes) => code.extend(bytes.iter().map(|b| b.to_u8())),
        }
    }

    assert_eq!(
        code.len(),
        offset,
        "The two phases gave different code lengths"
    );
}

/// The size of a `PushTarget`, in bytes.
fn push_target_size(target: &PushTarget) -> u8 {
    match target {
        PushTarget::Literal(lit) => lit.to_trimmed_be_bytes().len() as u8,
        PushTarget::Label(_) => BYTES_PER_OFFSET,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use itertools::Itertools;

    use crate::cpu::kernel::parser::parse;
    use crate::cpu::kernel::{assembler::*, ast::*};

    #[test]
    fn two_files() {
        // We will test two simple files, with a label and a jump, to ensure that jump offsets
        // are correctly shifted based on the offset of the containing file.

        let file_1 = File {
            body: vec![
                Item::GlobalLabelDeclaration("function_1".to_string()),
                Item::StandardOp("JUMPDEST".to_string()),
                Item::StandardOp("ADD".to_string()),
                Item::StandardOp("MUL".to_string()),
            ],
        };

        let file_2 = File {
            body: vec![
                Item::GlobalLabelDeclaration("function_2".to_string()),
                Item::StandardOp("JUMPDEST".to_string()),
                Item::StandardOp("DIV".to_string()),
                Item::LocalLabelDeclaration("mylabel".to_string()),
                Item::StandardOp("JUMPDEST".to_string()),
                Item::StandardOp("MOD".to_string()),
                Item::Push(PushTarget::Label("mylabel".to_string())),
                Item::StandardOp("JUMP".to_string()),
            ],
        };

        let expected_code = vec![
            get_opcode("JUMPDEST"),
            get_opcode("ADD"),
            get_opcode("MUL"),
            get_opcode("JUMPDEST"),
            get_opcode("DIV"),
            get_opcode("JUMPDEST"),
            get_opcode("MOD"),
            get_push_opcode(BYTES_PER_OFFSET),
            // The label offset, 5, in 3-byte BE form.
            0,
            0,
            5,
            get_opcode("JUMP"),
        ];

        let mut expected_global_labels = HashMap::new();
        expected_global_labels.insert("function_1".to_string(), 0);
        expected_global_labels.insert("function_2".to_string(), 3);

        let expected_kernel = Kernel {
            code: expected_code,
            global_labels: expected_global_labels,
        };

        let program = vec![file_1, file_2];
        assert_eq!(assemble(program), expected_kernel);
    }

    #[test]
    #[should_panic]
    fn global_label_collision() {
        let file_1 = File {
            body: vec![
                Item::GlobalLabelDeclaration("foo".to_string()),
                Item::StandardOp("JUMPDEST".to_string()),
            ],
        };
        let file_2 = File {
            body: vec![
                Item::GlobalLabelDeclaration("foo".to_string()),
                Item::StandardOp("JUMPDEST".to_string()),
            ],
        };
        assemble(vec![file_1, file_2]);
    }

    #[test]
    #[should_panic]
    fn local_label_collision() {
        let file = File {
            body: vec![
                Item::LocalLabelDeclaration("foo".to_string()),
                Item::StandardOp("JUMPDEST".to_string()),
                Item::LocalLabelDeclaration("foo".to_string()),
                Item::StandardOp("ADD".to_string()),
            ],
        };
        assemble(vec![file]);
    }

    #[test]
    fn literal_bytes() {
        let file = File {
            body: vec![
                Item::Bytes(vec![
                    Literal::Hex("12".to_string()),
                    Literal::Decimal("42".to_string()),
                ]),
                Item::Bytes(vec![
                    Literal::Hex("fe".to_string()),
                    Literal::Decimal("255".to_string()),
                ]),
            ],
        };
        let code = assemble(vec![file]).code;
        assert_eq!(code, vec![0x12, 42, 0xfe, 255]);
    }

    #[test]
    fn macro_in_macro() {
        let kernel = parse_and_assemble(&[
            "%macro foo %bar %bar %endmacro",
            "%macro bar ADD %endmacro",
            "%foo",
        ]);
        let add = get_opcode("ADD");
        assert_eq!(kernel.code, vec![add, add]);
    }

    fn parse_and_assemble(files: &[&str]) -> Kernel {
        let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
        assemble(parsed_files)
    }
}
