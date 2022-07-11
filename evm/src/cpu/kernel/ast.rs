use ethereum_types::U256;
use plonky2_util::ceil_div_usize;

#[derive(Debug)]
pub(crate) struct File {
    pub(crate) body: Vec<Item>,
}

#[derive(Clone, Debug)]
pub(crate) enum Item {
    /// Defines a new macro: name, params, body.
    MacroDef(String, Vec<String>, Vec<Item>),
    /// Calls a macro: name, args.
    MacroCall(String, Vec<PushTarget>),
    /// Declares a global label.
    GlobalLabelDeclaration(String),
    /// Declares a label that is local to the current file.
    LocalLabelDeclaration(String),
    /// A `PUSH` operation.
    Push(PushTarget),
    /// Any opcode besides a PUSH opcode.
    StandardOp(String),
    /// Literal hex data; should contain an even number of hex chars.
    Bytes(Vec<Literal>),
}

/// The target of a `PUSH` operation.
#[derive(Clone, Debug)]
pub(crate) enum PushTarget {
    Literal(Literal),
    Label(String),
    MacroVar(String),
    Constant(String),
}

#[derive(Clone, Debug)]
pub(crate) enum Literal {
    Decimal(String),
    Hex(String),
}

impl Literal {
    pub(crate) fn to_trimmed_be_bytes(&self) -> Vec<u8> {
        let u256 = self.to_u256();
        let num_bytes = ceil_div_usize(u256.bits(), 8).max(1);
        // `byte` is little-endian, so we manually reverse it.
        (0..num_bytes).rev().map(|i| u256.byte(i)).collect()
    }

    pub(crate) fn to_u256(&self) -> U256 {
        let (src, radix) = match self {
            Literal::Decimal(s) => (s, 10),
            Literal::Hex(s) => (s, 16),
        };
        U256::from_str_radix(src, radix)
            .unwrap_or_else(|_| panic!("Not a valid u256 literal: {:?}", self))
    }

    pub(crate) fn to_u8(&self) -> u8 {
        let (src, radix) = match self {
            Literal::Decimal(s) => (s, 10),
            Literal::Hex(s) => (s, 16),
        };
        u8::from_str_radix(src, radix)
            .unwrap_or_else(|_| panic!("Not a valid u8 literal: {:?}", self))
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::kernel::ast::*;

    #[test]
    fn literal_to_be_bytes() {
        assert_eq!(
            Literal::Decimal("0".into()).to_trimmed_be_bytes(),
            vec![0x00]
        );

        assert_eq!(
            Literal::Decimal("768".into()).to_trimmed_be_bytes(),
            vec![0x03, 0x00]
        );

        assert_eq!(
            Literal::Hex("a1b2".into()).to_trimmed_be_bytes(),
            vec![0xa1, 0xb2]
        );

        assert_eq!(
            Literal::Hex("1b2".into()).to_trimmed_be_bytes(),
            vec![0x1, 0xb2]
        );
    }
}
