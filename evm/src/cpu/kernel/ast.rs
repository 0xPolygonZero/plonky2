use ethereum_types::U256;

use crate::generation::prover_input::ProverInputFn;

#[derive(Debug)]
pub(crate) struct File {
    pub(crate) body: Vec<Item>,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) enum Item {
    /// Defines a new macro: name, params, body.
    MacroDef(String, Vec<String>, Vec<Item>),
    /// Calls a macro: name, args.
    MacroCall(String, Vec<PushTarget>),
    /// Repetition, like `%rep` in NASM.
    Repeat(U256, Vec<Item>),
    /// A directive to manipulate the stack according to a specified pattern.
    /// The first list gives names to items on the top of the stack.
    /// The second list specifies replacement items.
    /// Example: `(a, b, c) -> (c, 5, 0x20, @SOME_CONST, a)`.
    StackManipulation(Vec<StackPlaceholder>, Vec<StackReplacement>),
    /// Declares a global label.
    GlobalLabelDeclaration(String),
    /// Declares a label that is local to the current file.
    LocalLabelDeclaration(String),
    /// Declares a label that is local to the macro it's declared in.
    MacroLabelDeclaration(String),
    /// A `PUSH` operation.
    Push(PushTarget),
    /// A `ProverInput` operation.
    ProverInput(ProverInputFn),
    /// Any opcode besides a PUSH opcode.
    StandardOp(String),
    /// Literal hex data; should contain an even number of hex chars.
    Bytes(Vec<u8>),
}

/// The left hand side of a %stack stack-manipulation macro.
#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) enum StackPlaceholder {
    Identifier(String),
    Block(String, usize),
}

/// The right hand side of a %stack stack-manipulation macro.
#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) enum StackReplacement {
    Literal(U256),
    /// Can be either a named item or a label.
    Identifier(String),
    Label(String),
    MacroLabel(String),
    MacroVar(String),
    Constant(String),
}

impl From<PushTarget> for StackReplacement {
    fn from(target: PushTarget) -> Self {
        match target {
            PushTarget::Literal(x) => Self::Literal(x),
            PushTarget::Label(l) => Self::Label(l),
            PushTarget::MacroLabel(l) => Self::MacroLabel(l),
            PushTarget::MacroVar(v) => Self::MacroVar(v),
            PushTarget::Constant(c) => Self::Constant(c),
        }
    }
}

/// The target of a `PUSH` operation.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum PushTarget {
    Literal(U256),
    Label(String),
    MacroLabel(String),
    MacroVar(String),
    Constant(String),
}
