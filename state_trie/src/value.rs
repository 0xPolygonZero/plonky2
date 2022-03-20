use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::HashOut;
use primitive_types::H256;

use crate::account::Account;

type F = GoldilocksField;

#[derive(Debug)]
pub enum Value {
    Account(Account),
    Storage(H256),
    Transaction, // TODO: What should fields be?
    Receipt,     // TODO: What should fields be?
}

impl Value {
    pub(crate) fn hash(&self) -> HashOut<F> {
        todo!()
    }
}
