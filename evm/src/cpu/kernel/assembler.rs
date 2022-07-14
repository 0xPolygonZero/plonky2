use std::collections::HashMap;

use ethereum_types::U256;
use itertools::izip;
use log::debug;

use super::ast::PushTarget;
use crate::cpu::kernel::ast::Literal;
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
    pub(crate) code: Vec<u8>,
    pub(crate) global_labels: HashMap<String, usize>,
}

struct Macro {
    params: Vec<String>,
    items: Vec<Item>,
}

impl Macro {
    fn get_param_index(&self, param: &str) -> usize {
        self.params
            .iter()
            .position(|p| p == param)
            .unwrap_or_else(|| panic!("No such param: {} {:?}", param, &self.params))
    }
}

pub(crate) fn assemble(files: Vec<File>, constants: HashMap<String, U256>) -> Kernel {
    let macros = find_macros(&files);
    let mut global_labels = HashMap::new();
    let mut offset = 0;
    let mut expanded_files = Vec::with_capacity(files.len());
    let mut local_labels = Vec::with_capacity(files.len());
    for file in files {
        let expanded_file = expand_macros(file.body, &macros);
        let expanded_file = inline_constants(expanded_file, &constants);
        local_labels.push(find_labels(&expanded_file, &mut offset, &mut global_labels));
        expanded_files.push(expanded_file);
    }
    let mut code = vec![];
    for (file, locals) in izip!(expanded_files, local_labels) {
        let prev_len = code.len();
        assemble_file(file, &mut code, locals, &global_labels);
        let file_len = code.len() - prev_len;
        debug!("Assembled file size: {} bytes", file_len);
    }
    assert_eq!(code.len(), offset, "Code length doesn't match offset.");
    Kernel {
        code,
        global_labels,
    }
}

fn find_macros(files: &[File]) -> HashMap<String, Macro> {
    let mut macros = HashMap::new();
    for file in files {
        for item in &file.body {
            if let Item::MacroDef(name, params, items) = item {
                let _macro = Macro {
                    params: params.clone(),
                    items: items.clone(),
                };
                let old = macros.insert(name.clone(), _macro);
                assert!(old.is_none(), "Duplicate macro: {name}");
            }
        }
    }
    macros
}

fn expand_macros(body: Vec<Item>, macros: &HashMap<String, Macro>) -> Vec<Item> {
    let mut expanded = vec![];
    for item in body {
        match item {
            Item::MacroDef(_, _, _) => {
                // At this phase, we no longer need macro definitions.
            }
            Item::MacroCall(m, args) => {
                expanded.extend(expand_macro_call(m, args, macros));
            }
            item => {
                expanded.push(item);
            }
        }
    }
    expanded
}

fn expand_macro_call(
    name: String,
    args: Vec<PushTarget>,
    macros: &HashMap<String, Macro>,
) -> Vec<Item> {
    let _macro = macros
        .get(&name)
        .unwrap_or_else(|| panic!("No such macro: {}", name));

    assert_eq!(
        args.len(),
        _macro.params.len(),
        "Macro `{}`: expected {} arguments, got {}",
        name,
        _macro.params.len(),
        args.len()
    );

    let expanded_item = _macro
        .items
        .iter()
        .map(|item| {
            if let Item::Push(PushTarget::MacroVar(var)) = item {
                let param_index = _macro.get_param_index(var);
                Item::Push(args[param_index].clone())
            } else {
                item.clone()
            }
        })
        .collect();

    // Recursively expand any macros in the expanded code.
    expand_macros(expanded_item, macros)
}

fn inline_constants(body: Vec<Item>, constants: &HashMap<String, U256>) -> Vec<Item> {
    body.into_iter()
        .map(|item| {
            if let Item::Push(PushTarget::Constant(c)) = item {
                let value = constants
                    .get(&c)
                    .unwrap_or_else(|| panic!("No such constant: {}", c));
                let literal = Literal::Decimal(value.to_string());
                Item::Push(PushTarget::Literal(literal))
            } else {
                item
            }
        })
        .collect()
}

fn find_labels(
    body: &[Item],
    offset: &mut usize,
    global_labels: &mut HashMap<String, usize>,
) -> HashMap<String, usize> {
    // Discover the offset of each label in this file.
    let mut local_labels = HashMap::<String, usize>::new();
    for item in body {
        match item {
            Item::MacroDef(_, _, _) | Item::MacroCall(_, _) => {
                panic!("Macros should have been expanded already")
            }
            Item::GlobalLabelDeclaration(label) => {
                let old = global_labels.insert(label.clone(), *offset);
                assert!(old.is_none(), "Duplicate global label: {}", label);
            }
            Item::LocalLabelDeclaration(label) => {
                let old = local_labels.insert(label.clone(), *offset);
                assert!(old.is_none(), "Duplicate local label: {}", label);
            }
            Item::Push(target) => *offset += 1 + push_target_size(target) as usize,
            Item::StandardOp(_) => *offset += 1,
            Item::Bytes(bytes) => *offset += bytes.len(),
        }
    }
    local_labels
}

fn assemble_file(
    body: Vec<Item>,
    code: &mut Vec<u8>,
    local_labels: HashMap<String, usize>,
    global_labels: &HashMap<String, usize>,
) {
    // Assemble the file.
    for item in body {
        match item {
            Item::MacroDef(_, _, _) | Item::MacroCall(_, _) => {
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
                    PushTarget::MacroVar(v) => panic!("Variable not in a macro: {}", v),
                    PushTarget::Constant(c) => panic!("Constant wasn't inlined: {}", c),
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
}

/// The size of a `PushTarget`, in bytes.
fn push_target_size(target: &PushTarget) -> u8 {
    match target {
        PushTarget::Literal(lit) => lit.to_trimmed_be_bytes().len() as u8,
        PushTarget::Label(_) => BYTES_PER_OFFSET,
        PushTarget::MacroVar(v) => panic!("Variable not in a macro: {}", v),
        PushTarget::Constant(c) => panic!("Constant wasn't inlined: {}", c),
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
        assert_eq!(assemble(program, HashMap::new()), expected_kernel);
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
        assemble(vec![file_1, file_2], HashMap::new());
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
        assemble(vec![file], HashMap::new());
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
        let code = assemble(vec![file], HashMap::new()).code;
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

    #[test]
    fn macro_with_vars() {
        let kernel = parse_and_assemble(&[
            "%macro add(x, y) PUSH $x PUSH $y ADD %endmacro",
            "%add(2, 3)",
        ]);
        let push1 = get_push_opcode(1);
        let add = get_opcode("ADD");
        assert_eq!(kernel.code, vec![push1, 2, push1, 3, add]);
    }

    #[test]
    #[should_panic]
    fn macro_with_wrong_vars() {
        parse_and_assemble(&[
            "%macro add(x, y) PUSH $x PUSH $y ADD %endmacro",
            "%add(2, 3, 4)",
        ]);
    }

    #[test]
    #[should_panic]
    fn var_not_in_macro() {
        parse_and_assemble(&["push $abc"]);
    }

    #[test]
    fn constants() {
        let code = &["PUSH @DEAD_BEEF"];
        let mut constants = HashMap::new();
        constants.insert("DEAD_BEEF".into(), 0xDEADBEEFu64.into());

        let kernel = parse_and_assemble_with_constants(code, constants);
        let push4 = get_push_opcode(4);
        assert_eq!(kernel.code, vec![push4, 0xDE, 0xAD, 0xBE, 0xEF]);
    }

    fn parse_and_assemble(files: &[&str]) -> Kernel {
        parse_and_assemble_with_constants(files, HashMap::new())
    }

    fn parse_and_assemble_with_constants(
        files: &[&str],
        constants: HashMap<String, U256>,
    ) -> Kernel {
        let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
        assemble(parsed_files, constants)
    }
}
