use std::collections::HashMap;

use ethereum_types::U256;
use itertools::izip;
use log::debug;
use plonky2_util::ceil_div_usize;

use super::ast::PushTarget;
use crate::cpu::kernel::ast::Item::LocalLabelDeclaration;
use crate::cpu::kernel::ast::StackReplacement;
use crate::cpu::kernel::keccak_util::hash_kernel;
use crate::cpu::kernel::optimizer::optimize_asm;
use crate::cpu::kernel::stack::stack_manipulation::expand_stack_manipulation;
use crate::cpu::kernel::utils::u256_to_trimmed_be_bytes;
use crate::cpu::kernel::{
    ast::{File, Item},
    opcodes::{get_opcode, get_push_opcode},
};
use crate::generation::prover_input::ProverInputFn;
use crate::keccak_sponge::columns::KECCAK_RATE_BYTES;

/// The number of bytes to push when pushing an offset within the code (i.e. when assembling jumps).
/// Ideally we would automatically use the minimal number of bytes required, but that would be
/// nontrivial given the circular dependency between an offset and its size.
pub(crate) const BYTES_PER_OFFSET: u8 = 3;

#[derive(PartialEq, Eq, Debug)]
pub struct Kernel {
    pub(crate) code: Vec<u8>,

    /// Computed using `hash_kernel`. It is encoded as `u32` limbs for convenience, since we deal
    /// with `u32` limbs in our Keccak table.
    pub(crate) code_hash: [u32; 8],

    pub(crate) global_labels: HashMap<String, usize>,

    /// Map from `PROVER_INPUT` offsets to their corresponding `ProverInputFn`.
    pub(crate) prover_inputs: HashMap<usize, ProverInputFn>,
}

impl Kernel {
    fn new(
        code: Vec<u8>,
        global_labels: HashMap<String, usize>,
        prover_inputs: HashMap<usize, ProverInputFn>,
    ) -> Self {
        let code_hash = hash_kernel(&Self::padded_code_helper(&code));

        Self {
            code,
            code_hash,
            global_labels,
            prover_inputs,
        }
    }

    /// Zero-pads the code such that its length is a multiple of the Keccak rate.
    pub(crate) fn padded_code(&self) -> Vec<u8> {
        Self::padded_code_helper(&self.code)
    }

    fn padded_code_helper(code: &[u8]) -> Vec<u8> {
        let padded_len = ceil_div_usize(code.len(), KECCAK_RATE_BYTES) * KECCAK_RATE_BYTES;
        let mut padded_code = code.to_vec();
        padded_code.resize(padded_len, 0);
        padded_code
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
struct MacroSignature {
    name: String,
    num_params: usize,
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
            .unwrap_or_else(|| panic!("No such param: {param} {:?}", &self.params))
    }
}

pub(crate) fn assemble(
    files: Vec<File>,
    constants: HashMap<String, U256>,
    optimize: bool,
) -> Kernel {
    let macros = find_macros(&files);
    let mut global_labels = HashMap::new();
    let mut prover_inputs = HashMap::new();
    let mut offset = 0;
    let mut expanded_files = Vec::with_capacity(files.len());
    let mut local_labels = Vec::with_capacity(files.len());
    let mut macro_counter = 0;
    for file in files {
        let mut file = file.body;
        file = expand_macros(file, &macros, &mut macro_counter);
        file = inline_constants(file, &constants);
        file = expand_stack_manipulation(file);
        if optimize {
            optimize_asm(&mut file);
        }
        local_labels.push(find_labels(
            &file,
            &mut offset,
            &mut global_labels,
            &mut prover_inputs,
        ));
        expanded_files.push(file);
    }
    let mut code = vec![];
    for (file, locals) in izip!(expanded_files, local_labels) {
        let prev_len = code.len();
        assemble_file(file, &mut code, locals, &global_labels);
        let file_len = code.len() - prev_len;
        debug!("Assembled file size: {} bytes", file_len);
    }
    assert_eq!(code.len(), offset, "Code length doesn't match offset.");
    Kernel::new(code, global_labels, prover_inputs)
}

fn find_macros(files: &[File]) -> HashMap<MacroSignature, Macro> {
    let mut macros = HashMap::new();
    for file in files {
        for item in &file.body {
            if let Item::MacroDef(name, params, items) = item {
                let signature = MacroSignature {
                    name: name.clone(),
                    num_params: params.len(),
                };
                let macro_ = Macro {
                    params: params.clone(),
                    items: items.clone(),
                };
                let old = macros.insert(signature.clone(), macro_);
                assert!(old.is_none(), "Duplicate macro signature: {signature:?}");
            }
        }
    }
    macros
}

fn expand_macros(
    body: Vec<Item>,
    macros: &HashMap<MacroSignature, Macro>,
    macro_counter: &mut u32,
) -> Vec<Item> {
    let mut expanded = vec![];
    for item in body {
        match item {
            Item::MacroDef(_, _, _) => {
                // At this phase, we no longer need macro definitions.
            }
            Item::MacroCall(m, args) => {
                expanded.extend(expand_macro_call(m, args, macros, macro_counter));
            }
            Item::Repeat(count, body) => {
                for _ in 0..count.as_usize() {
                    expanded.extend(expand_macros(body.clone(), macros, macro_counter));
                }
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
    macros: &HashMap<MacroSignature, Macro>,
    macro_counter: &mut u32,
) -> Vec<Item> {
    let signature = MacroSignature {
        name,
        num_params: args.len(),
    };
    let macro_ = macros
        .get(&signature)
        .unwrap_or_else(|| panic!("No such macro: {signature:?}"));

    let get_actual_label = |macro_label| format!("@{macro_counter}.{macro_label}");

    let get_arg = |var| {
        let param_index = macro_.get_param_index(var);
        args[param_index].clone()
    };

    let expanded_item = macro_
        .items
        .iter()
        .map(|item| match item {
            Item::MacroLabelDeclaration(label) => LocalLabelDeclaration(get_actual_label(label)),
            Item::Push(PushTarget::MacroLabel(label)) => {
                Item::Push(PushTarget::Label(get_actual_label(label)))
            }
            Item::Push(PushTarget::MacroVar(var)) => Item::Push(get_arg(var)),
            Item::MacroCall(name, args) => {
                let expanded_args = args
                    .iter()
                    .map(|arg| match arg {
                        PushTarget::MacroVar(var) => get_arg(var),
                        PushTarget::MacroLabel(l) => PushTarget::Label(get_actual_label(l)),
                        _ => arg.clone(),
                    })
                    .collect();
                Item::MacroCall(name.clone(), expanded_args)
            }
            Item::StackManipulation(before, after) => {
                let after = after
                    .iter()
                    .map(|replacement| match replacement {
                        StackReplacement::MacroLabel(label) => {
                            StackReplacement::Identifier(get_actual_label(label))
                        }
                        StackReplacement::MacroVar(var) => get_arg(var).into(),
                        _ => replacement.clone(),
                    })
                    .collect();
                Item::StackManipulation(before.clone(), after)
            }
            _ => item.clone(),
        })
        .collect();

    *macro_counter += 1;

    // Recursively expand any macros in the expanded code.
    expand_macros(expanded_item, macros, macro_counter)
}

fn inline_constants(body: Vec<Item>, constants: &HashMap<String, U256>) -> Vec<Item> {
    let resolve_const = |c| {
        *constants
            .get(&c)
            .unwrap_or_else(|| panic!("No such constant: {c}"))
    };

    body.into_iter()
        .map(|item| {
            if let Item::Push(PushTarget::Constant(c)) = item {
                Item::Push(PushTarget::Literal(resolve_const(c)))
            } else if let Item::StackManipulation(from, to) = item {
                let to = to
                    .into_iter()
                    .map(|replacement| {
                        if let StackReplacement::Constant(c) = replacement {
                            StackReplacement::Literal(resolve_const(c))
                        } else {
                            replacement
                        }
                    })
                    .collect();
                Item::StackManipulation(from, to)
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
    prover_inputs: &mut HashMap<usize, ProverInputFn>,
) -> HashMap<String, usize> {
    // Discover the offset of each label in this file.
    let mut local_labels = HashMap::<String, usize>::new();
    for item in body {
        match item {
            Item::MacroDef(_, _, _)
            | Item::MacroCall(_, _)
            | Item::Repeat(_, _)
            | Item::StackManipulation(_, _)
            | Item::MacroLabelDeclaration(_) => {
                panic!("Item should have been expanded already: {item:?}");
            }
            Item::GlobalLabelDeclaration(label) => {
                let old = global_labels.insert(label.clone(), *offset);
                assert!(old.is_none(), "Duplicate global label: {label}");
            }
            Item::LocalLabelDeclaration(label) => {
                let old = local_labels.insert(label.clone(), *offset);
                assert!(old.is_none(), "Duplicate local label: {label}");
            }
            Item::Push(target) => *offset += 1 + push_target_size(target) as usize,
            Item::ProverInput(prover_input_fn) => {
                prover_inputs.insert(*offset, prover_input_fn.clone());
                *offset += 1;
            }
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
            Item::MacroDef(_, _, _)
            | Item::MacroCall(_, _)
            | Item::Repeat(_, _)
            | Item::StackManipulation(_, _)
            | Item::MacroLabelDeclaration(_) => {
                panic!("Item should have been expanded already: {item:?}");
            }
            Item::GlobalLabelDeclaration(_) | Item::LocalLabelDeclaration(_) => {
                // Nothing to do; we processed labels in the prior phase.
            }
            Item::Push(target) => {
                let target_bytes: Vec<u8> = match target {
                    PushTarget::Literal(n) => u256_to_trimmed_be_bytes(&n),
                    PushTarget::Label(label) => {
                        let offset = local_labels
                            .get(&label)
                            .or_else(|| global_labels.get(&label))
                            .unwrap_or_else(|| panic!("No such label: {label}"));
                        // We want the BYTES_PER_OFFSET least significant bytes in BE order.
                        // It's easiest to rev the first BYTES_PER_OFFSET bytes of the LE encoding.
                        (0..BYTES_PER_OFFSET)
                            .rev()
                            .map(|i| offset.to_le_bytes()[i as usize])
                            .collect()
                    }
                    PushTarget::MacroLabel(v) => panic!("Macro label not in a macro: {v}"),
                    PushTarget::MacroVar(v) => panic!("Variable not in a macro: {v}"),
                    PushTarget::Constant(c) => panic!("Constant wasn't inlined: {c}"),
                };
                code.push(get_push_opcode(target_bytes.len() as u8));
                code.extend(target_bytes);
            }
            Item::ProverInput(_) => {
                code.push(get_opcode("PROVER_INPUT"));
            }
            Item::StandardOp(opcode) => {
                code.push(get_opcode(&opcode));
            }
            Item::Bytes(bytes) => code.extend(bytes),
        }
    }
}

/// The size of a `PushTarget`, in bytes.
fn push_target_size(target: &PushTarget) -> u8 {
    match target {
        PushTarget::Literal(n) => u256_to_trimmed_be_bytes(n).len() as u8,
        PushTarget::Label(_) => BYTES_PER_OFFSET,
        PushTarget::MacroLabel(v) => panic!("Macro label not in a macro: {v}"),
        PushTarget::MacroVar(v) => panic!("Variable not in a macro: {v}"),
        PushTarget::Constant(c) => panic!("Constant wasn't inlined: {c}"),
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

        let expected_kernel = Kernel::new(expected_code, expected_global_labels, HashMap::new());

        let program = vec![file_1, file_2];
        assert_eq!(assemble(program, HashMap::new(), false), expected_kernel);
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
        assemble(vec![file_1, file_2], HashMap::new(), false);
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
        assemble(vec![file], HashMap::new(), false);
    }

    #[test]
    fn literal_bytes() {
        let file = File {
            body: vec![Item::Bytes(vec![0x12, 42]), Item::Bytes(vec![0xFE, 255])],
        };
        let code = assemble(vec![file], HashMap::new(), false).code;
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
        let files = &[
            "%macro add(x, y) PUSH $x PUSH $y ADD %endmacro",
            "%add(2, 3)",
        ];
        let kernel = parse_and_assemble_ext(files, HashMap::new(), false);
        let push1 = get_push_opcode(1);
        let add = get_opcode("ADD");
        assert_eq!(kernel.code, vec![push1, 2, push1, 3, add]);
    }

    #[test]
    fn macro_with_label() {
        let files = &[
            "%macro jump(x) PUSH $x JUMP %endmacro",
            "%macro spin %%start: %jump(%%start) %endmacro",
            "%spin %spin",
        ];
        let kernel = parse_and_assemble_ext(files, HashMap::new(), false);
        let push3 = get_push_opcode(BYTES_PER_OFFSET);
        let jump = get_opcode("JUMP");
        assert_eq!(
            kernel.code,
            vec![push3, 0, 0, 0, jump, push3, 0, 0, 5, jump]
        );
    }

    #[test]
    fn macro_in_macro_with_vars() {
        let kernel = parse_and_assemble(&[
            "%macro foo(x) %bar($x) %bar($x) %endmacro",
            "%macro bar(y) PUSH $y %endmacro",
            "%foo(42)",
        ]);
        let push1 = get_push_opcode(1);
        assert_eq!(kernel.code, vec![push1, 42, push1, 42]);
    }

    #[test]
    fn macro_with_reserved_prefix() {
        // The name `repeat` should be allowed, even though `rep` is reserved.
        parse_and_assemble(&["%macro repeat %endmacro", "%repeat"]);
    }

    #[test]
    fn overloaded_macros() {
        let kernel = parse_and_assemble(&[
            "%macro push(x) PUSH $x %endmacro",
            "%macro push(x, y) PUSH $x PUSH $y %endmacro",
            "%push(5)",
            "%push(6, 7)",
        ]);
        let push1 = get_push_opcode(1);
        assert_eq!(kernel.code, vec![push1, 5, push1, 6, push1, 7]);
    }

    #[test]
    fn pop2_macro() {
        parse_and_assemble(&["%macro pop2 %rep 2 pop %endrep %endmacro", "%pop2"]);
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

        let kernel = parse_and_assemble_ext(code, constants, true);
        let push4 = get_push_opcode(4);
        assert_eq!(kernel.code, vec![push4, 0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn repeat() {
        let kernel = parse_and_assemble(&["%rep 3 ADD %endrep"]);
        let add = get_opcode("ADD");
        assert_eq!(kernel.code, vec![add, add, add]);
    }

    #[test]
    fn stack_manipulation() {
        let pop = get_opcode("POP");
        let dup1 = get_opcode("DUP1");
        let swap1 = get_opcode("SWAP1");
        let swap2 = get_opcode("SWAP2");
        let swap3 = get_opcode("SWAP3");
        let push_one_byte = get_push_opcode(1);
        let push_label = get_push_opcode(BYTES_PER_OFFSET);

        let kernel = parse_and_assemble(&["%stack () -> (1, 2, 3)"]);
        assert_eq!(
            kernel.code,
            vec![push_one_byte, 3, push_one_byte, 2, push_one_byte, 1]
        );

        let kernel = parse_and_assemble(&["%stack (a) -> (a)"]);
        assert_eq!(kernel.code, vec![] as Vec<u8>);

        let kernel = parse_and_assemble(&["%stack (a, b, c) -> (c, b, a)"]);
        assert_eq!(kernel.code, vec![swap2]);

        let kernel = parse_and_assemble(&["%stack (a, b, c) -> (b)"]);
        assert_eq!(kernel.code, vec![pop, swap1, pop]);

        let kernel = parse_and_assemble(&["%stack (a, b, c) -> (7, b)"]);
        assert_eq!(kernel.code, vec![pop, swap1, pop, push_one_byte, 7]);

        let kernel = parse_and_assemble(&["%stack (a, b: 3, c) -> (c)"]);
        assert_eq!(kernel.code, vec![pop, pop, pop, pop]);

        let kernel = parse_and_assemble(&["%stack (a: 2, b: 2) -> (b, a)"]);
        assert_eq!(kernel.code, vec![swap1, swap3, swap1, swap2]);

        let kernel1 = parse_and_assemble(&["%stack (a: 3, b: 3, c) -> (c, b, a)"]);
        let kernel2 =
            parse_and_assemble(&["%stack (a, b, c, d, e, f, g) -> (g, d, e, f, a, b, c)"]);
        assert_eq!(kernel1.code, kernel2.code);

        let mut consts = HashMap::new();
        consts.insert("LIFE".into(), 42.into());
        parse_and_assemble_ext(&["%stack (a, b) -> (b, @LIFE)"], consts, true);
        // We won't check the code since there are two equally efficient implementations.

        let kernel = parse_and_assemble(&["start: %stack (a, b) -> (start)"]);
        assert_eq!(kernel.code, vec![pop, pop, push_label, 0, 0, 0]);

        // The "start" label gets shadowed by the "start" named stack item.
        let kernel = parse_and_assemble(&["start: %stack (start) -> (start, start)"]);
        assert_eq!(kernel.code, vec![dup1]);
    }

    #[test]
    fn stack_manipulation_in_macro() {
        let pop = get_opcode("POP");
        let push1 = get_push_opcode(1);

        let kernel = parse_and_assemble(&[
            "%macro set_top(x) %stack (a) -> ($x) %endmacro",
            "%set_top(42)",
        ]);
        assert_eq!(kernel.code, vec![pop, push1, 42]);
    }

    #[test]
    fn stack_manipulation_in_macro_with_name_collision() {
        let pop = get_opcode("POP");
        let push_label = get_push_opcode(BYTES_PER_OFFSET);

        // In the stack directive, there's a named item `foo`.
        // But when we invoke `%foo(foo)`, the argument refers to the `foo` label.
        // Thus the expanded macro is `%stack (foo) -> (label foo)` (not real syntax).
        let kernel = parse_and_assemble(&[
            "global foo:",
            "%macro foo(x) %stack (foo) -> ($x) %endmacro",
            "%foo(foo)",
        ]);
        assert_eq!(kernel.code, vec![pop, push_label, 0, 0, 0]);
    }

    fn parse_and_assemble(files: &[&str]) -> Kernel {
        parse_and_assemble_ext(files, HashMap::new(), true)
    }

    fn parse_and_assemble_ext(
        files: &[&str],
        constants: HashMap<String, U256>,
        optimize: bool,
    ) -> Kernel {
        let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
        assemble(parsed_files, constants, optimize)
    }
}
