use anyhow::bail;
use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::HashOut;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::Hasher;
use primitive_types::U256;

use crate::util::u256_to_u32s_le_f;

type F = GoldilocksField;

#[derive(Debug)]
pub struct Account {
    pub(crate) nonce: F,
    pub(crate) balance: U256,
    pub(crate) code: Vec<F>, // TODO: Split off?
}

impl Account {
    fn digest(&self) -> HashOut<F> {
        let code_digest = PoseidonHash::hash_pad(&self.code);
        PoseidonHash::hash_no_pad(
            &[
                [self.nonce].as_slice(),
                &u256_to_u32s_le_f(&self.balance),
                &code_digest.elements,
            ]
            .concat(),
        )
    }

    pub(crate) fn with_value_added(&self, value: U256) -> Self {
        Account {
            balance: prev_acc.balance + value,
            ..prev_acc
        }
    }

    pub(crate) fn with_value_subtracted(&self, value: U256) -> anyhow::Result<Self> {
        if value > self.balance {
            bail!("Insufficient balance");
        }

        Ok(Account {
            balance: prev_acc.balance - value,
            ..prev_acc
        })
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            balance: U256::zero(),
            code: vec![],
            nonce: F::ZERO,
        }
    }
}
