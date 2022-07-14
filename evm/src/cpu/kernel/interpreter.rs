use ethereum_types::{U256, U512};

/// Halt interpreter execution whenever a jump to this offset is done.
const HALT_OFFSET: usize = 0xdeadbeef;

struct Interpreter<'a> {
    code: &'a [u8],
    jumpdests: Vec<usize>,
    offset: usize,
    stack: Vec<U256>,
    running: bool,
}

pub fn run(code: &[u8], initial_offset: usize, initial_stack: Vec<U256>) -> Vec<U256> {
    let mut interpreter = Interpreter {
        code,
        jumpdests: find_jumpdests(code),
        offset: initial_offset,
        stack: initial_stack,
        running: true,
    };

    while interpreter.running {
        interpreter.run_opcode();
    }

    interpreter.stack
}

impl<'a> Interpreter<'a> {
    fn slice(&self, n: usize) -> &[u8] {
        &self.code[self.offset..self.offset + n]
    }

    fn incr(&mut self, n: usize) {
        self.offset += n;
    }

    fn push(&mut self, x: U256) {
        self.stack.push(x);
    }

    fn push_bool(&mut self, x: bool) {
        self.stack.push(if x { U256::one() } else { U256::zero() });
    }

    fn pop(&mut self) -> U256 {
        self.stack.pop().expect("Pop on empty stack.")
    }

    fn run_opcode(&mut self) {
        let opcode = self.code.get(self.offset).copied().unwrap_or_default();
        self.incr(1);
        match opcode {
            0x00 => self.run_stop(),                                   // "STOP",
            0x01 => self.run_add(),                                    // "ADD",
            0x02 => self.run_mul(),                                    // "MUL",
            0x03 => self.run_sub(),                                    // "SUB",
            0x04 => self.run_div(),                                    // "DIV",
            0x05 => todo!(),                                           // "SDIV",
            0x06 => self.run_mod(),                                    // "MOD",
            0x07 => todo!(),                                           // "SMOD",
            0x08 => self.run_addmod(),                                 // "ADDMOD",
            0x09 => self.run_mulmod(),                                 // "MULMOD",
            0x0a => self.run_exp(),                                    // "EXP",
            0x0b => todo!(),                                           // "SIGNEXTEND",
            0x10 => self.run_lt(),                                     // "LT",
            0x11 => self.run_gt(),                                     // "GT",
            0x12 => todo!(),                                           // "SLT",
            0x13 => todo!(),                                           // "SGT",
            0x14 => self.run_eq(),                                     // "EQ",
            0x15 => self.run_iszero(),                                 // "ISZERO",
            0x16 => self.run_and(),                                    // "AND",
            0x17 => self.run_or(),                                     // "OR",
            0x18 => self.run_xor(),                                    // "XOR",
            0x19 => self.run_not(),                                    // "NOT",
            0x1a => todo!(),                                           // "BYTE",
            0x1b => todo!(),                                           // "SHL",
            0x1c => todo!(),                                           // "SHR",
            0x1d => todo!(),                                           // "SAR",
            0x20 => todo!(),                                           // "KECCAK256",
            0x30 => todo!(),                                           // "ADDRESS",
            0x31 => todo!(),                                           // "BALANCE",
            0x32 => todo!(),                                           // "ORIGIN",
            0x33 => todo!(),                                           // "CALLER",
            0x34 => todo!(),                                           // "CALLVALUE",
            0x35 => todo!(),                                           // "CALLDATALOAD",
            0x36 => todo!(),                                           // "CALLDATASIZE",
            0x37 => todo!(),                                           // "CALLDATACOPY",
            0x38 => todo!(),                                           // "CODESIZE",
            0x39 => todo!(),                                           // "CODECOPY",
            0x3a => todo!(),                                           // "GASPRICE",
            0x3b => todo!(),                                           // "EXTCODESIZE",
            0x3c => todo!(),                                           // "EXTCODECOPY",
            0x3d => todo!(),                                           // "RETURNDATASIZE",
            0x3e => todo!(),                                           // "RETURNDATACOPY",
            0x3f => todo!(),                                           // "EXTCODEHASH",
            0x40 => todo!(),                                           // "BLOCKHASH",
            0x41 => todo!(),                                           // "COINBASE",
            0x42 => todo!(),                                           // "TIMESTAMP",
            0x43 => todo!(),                                           // "NUMBER",
            0x44 => todo!(),                                           // "DIFFICULTY",
            0x45 => todo!(),                                           // "GASLIMIT",
            0x46 => todo!(),                                           // "CHAINID",
            0x48 => todo!(),                                           // "BASEFEE",
            0x50 => self.run_pop(),                                    // "POP",
            0x51 => todo!(),                                           // "MLOAD",
            0x52 => todo!(),                                           // "MSTORE",
            0x53 => todo!(),                                           // "MSTORE8",
            0x54 => todo!(),                                           // "SLOAD",
            0x55 => todo!(),                                           // "SSTORE",
            0x56 => self.run_jump(),                                   // "JUMP",
            0x57 => self.run_jumpi(),                                  // "JUMPI",
            0x58 => todo!(),                                           // "GETPC",
            0x59 => todo!(),                                           // "MSIZE",
            0x5a => todo!(),                                           // "GAS",
            0x5b => (),                                                // "JUMPDEST",
            x if (0x60..0x80).contains(&x) => self.run_push(x - 0x5f), // "PUSH"
            x if (0x80..0x90).contains(&x) => self.run_dup(x - 0x7f),  // "DUP"
            x if (0x90..0xa0).contains(&x) => self.run_swap(x - 0x8f), // "SWAP"
            0xa0 => todo!(),                                           // "LOG0",
            0xa1 => todo!(),                                           // "LOG1",
            0xa2 => todo!(),                                           // "LOG2",
            0xa3 => todo!(),                                           // "LOG3",
            0xa4 => todo!(),                                           // "LOG4",
            0xf0 => todo!(),                                           // "CREATE",
            0xf1 => todo!(),                                           // "CALL",
            0xf2 => todo!(),                                           // "CALLCODE",
            0xf3 => todo!(),                                           // "RETURN",
            0xf4 => todo!(),                                           // "DELEGATECALL",
            0xf5 => todo!(),                                           // "CREATE2",
            0xfa => todo!(),                                           // "STATICCALL",
            0xfd => todo!(),                                           // "REVERT",
            0xfe => todo!(),                                           // "INVALID",
            0xff => todo!(),                                           // "SELFDESTRUCT",
            _ => panic!("Unrecognized opcode {}.", opcode),
        };
    }

    fn run_stop(&mut self) {
        self.running = false;
    }

    fn run_add(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_add(y).0);
    }

    fn run_mul(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_mul(y).0);
    }

    fn run_sub(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_sub(y).0);
    }

    fn run_div(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(if y.is_zero() { U256::zero() } else { x / y });
    }

    fn run_mod(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(if y.is_zero() { U256::zero() } else { x % y });
    }

    fn run_addmod(&mut self) {
        let x = U512::from(self.pop());
        let y = U512::from(self.pop());
        let z = U512::from(self.pop());
        self.push(if z.is_zero() {
            U256::zero()
        } else {
            U256::try_from((x + y) % z).unwrap()
        });
    }

    fn run_mulmod(&mut self) {
        let x = self.pop();
        let y = self.pop();
        let z = U512::from(self.pop());
        self.push(if z.is_zero() {
            U256::zero()
        } else {
            U256::try_from(x.full_mul(y) % z).unwrap()
        });
    }

    fn run_exp(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_pow(y).0);
    }

    fn run_lt(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push_bool(x < y);
    }

    fn run_gt(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push_bool(x > y);
    }

    fn run_eq(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push_bool(x == y);
    }

    fn run_iszero(&mut self) {
        let x = self.pop();
        self.push_bool(x.is_zero());
    }

    fn run_and(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x & y);
    }

    fn run_or(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x | y);
    }

    fn run_xor(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x ^ y);
    }

    fn run_not(&mut self) {
        let x = self.pop();
        self.push(!x);
    }

    fn run_pop(&mut self) {
        self.pop();
    }

    fn run_jump(&mut self) {
        let x = self.pop().as_usize();
        self.offset = x;
        if self.offset == HALT_OFFSET {
            self.running = false;
        } else if self.jumpdests.binary_search(&self.offset).is_err() {
            panic!("Destination is not a JUMPDEST.");
        }
    }

    fn run_jumpi(&mut self) {
        let x = self.pop().as_usize();
        let b = self.pop();
        if !b.is_zero() {
            self.offset = x;
            if self.offset == HALT_OFFSET {
                self.running = false;
            } else if self.jumpdests.binary_search(&self.offset).is_err() {
                panic!("Destination is not a JUMPDEST.");
            }
        }
    }

    fn run_push(&mut self, num_bytes: u8) {
        let x = U256::from_big_endian(self.slice(num_bytes as usize));
        self.incr(num_bytes as usize);
        self.push(x);
    }

    fn run_dup(&mut self, n: u8) {
        self.push(self.stack[self.stack.len() - n as usize]);
    }

    fn run_swap(&mut self, n: u8) {
        let len = self.stack.len();
        self.stack.swap(len - 1, len - n as usize - 1);
    }
}

/// Return the (ordered) JUMPDEST offsets in the code.
fn find_jumpdests(code: &[u8]) -> Vec<usize> {
    let mut offset = 0;
    let mut res = Vec::new();
    while offset < code.len() {
        let opcode = code[offset];
        match opcode {
            0x5b => res.push(offset),
            x if (0x60..0x80).contains(&x) => offset += x as usize - 0x5f, // PUSH instruction, disregard data.
            _ => (),
        }
        offset += 1;
    }
    res
}

#[cfg(test)]
mod tests {
    use crate::cpu::kernel::interpreter::run;

    #[test]
    fn test_run() {
        let code = vec![
            0x60, 0x1, 0x60, 0x2, 0x1, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56,
        ]; // PUSH1, 1, PUSH1, 2, ADD, PUSH4 deadbeef, JUMP
        assert_eq!(run(&code, 0, vec![]), vec![0x3.into()]);
    }
}
