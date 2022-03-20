use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::HashOut;
use primitive_types::{H160, H256};
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::Hasher;

use crate::util::{h160_to_u32s_le_f, h256_to_u32s_le_f};

type F = GoldilocksField;

#[derive(Debug)]
pub enum Key {
    Account(H160),
    Storage(H160, H256),
    Transaction, // TODO: What should fields be?
    Receipt,     // TODO: What should fields be?
}

impl Key {
    fn type_id(&self) -> F {
        F::from_canonical_usize(match self {
            Key::Account(_) => 1,
            Key::Storage(_, _) => 2,
            Key::Transaction => 3,
            Key::Receipt => 4,
        })
    }
}

impl Key {
    pub fn hash(&self) -> HashOut<F> {
        // Note: For security, we must ensure that no collisions are possible (without breaking the
        // underlying hash's collision resistance).
        // TODO: Explain our approach...

        let mut hash_in = vec![self.type_id()];
        match self {
            Key::Account(addr) => {
                hash_in.extend(h160_to_u32s_le_f(addr));
            }
            Key::Storage(addr, key) => {
                hash_in.extend(h160_to_u32s_le_f(addr));
                hash_in.extend(h256_to_u32s_le_f(key));
            }
            Key::Transaction => {
                todo!();
            }
            Key::Receipt => {
                todo!();
            }
        };

        PoseidonHash::hash_no_pad(&hash_in)
    }
}
