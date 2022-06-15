use ethereum_types::U256;
use plonky2_util::ceil_div_usize;

#[derive(Debug)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) body: Vec<Item>,
}

#[derive(Debug)]
pub(crate) enum Item {
    /// Declares a label that is local to the current function.
    LabelDeclaration(String),
    /// A `PUSH` operation.
    Push(PushTarget),
    /// Any opcode besides a PUSH opcode.
    StandardOp(String),
    /// Literal hex data; should contain an even number of hex chars.
    Literal(HexStr),
}

/// The target of a `PUSH` operation.
#[derive(Debug)]
pub(crate) enum PushTarget {
    Literal(Literal),
    Label(String),
}

#[derive(Debug)]
pub(crate) enum Literal {
    Decimal(String),
    Hex(HexStr),
}

impl Literal {
    pub(crate) fn to_trimmed_be_bytes(&self) -> Vec<u8> {
        let u256 = self.to_u256();
        let num_bytes = ceil_div_usize(u256.bits(), 8);
        // `byte` is little-endian, so we manually reverse it.
        (0..num_bytes).rev().map(|i| u256.byte(i)).collect()
    }

    pub(crate) fn to_u256(&self) -> U256 {
        match self {
            Literal::Decimal(dec) => U256::from_dec_str(dec).expect("Bad decimal string"),
            Literal::Hex(hex) => U256::from_big_endian(&hex.to_bytes()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct HexStr {
    pub(crate) nibbles: String,
}

impl HexStr {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        assert_eq!(
            self.nibbles.len() % 2,
            0,
            "Odd number of nibbles in hex string"
        );
        (0..self.nibbles.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&self.nibbles[i..i + 2], 16).expect("Hex nibble out of range")
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::kernel::ast::*;

    #[test]
    fn literal_to_be_bytes() {
        assert_eq!(
            Literal::Decimal("768".into()).to_trimmed_be_bytes(),
            vec![0x03, 0x00]
        );

        assert_eq!(
            Literal::Hex(HexStr {
                nibbles: "a1b2".into()
            })
            .to_trimmed_be_bytes(),
            vec![0xa1, 0xb2]
        );
    }
}
