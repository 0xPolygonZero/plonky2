use std::convert::identity;
use anyhow::bail;

use plonky2::field::goldilocks_field::GoldilocksField;
use primitive_types::{H160, H256, U256};

use crate::account::Account;
use crate::key::Key;
use crate::value::Value;

type F = GoldilocksField;

/// A Merkle trie for holding accounts and other blockchain state.
pub trait StateTrie {
    /// Returns the previous value, if any.
    fn update_opt_value<F>(&self, key: Key, f: F) -> Option<Value> // TODO: Return Merkle proofs.
    where
        F: FnMut(Option<Value>) -> Option<Value>;

    /// Returns the previous account state, if any.
    fn update_opt_account<F>(&self, addr: H160, mut f: F) -> Option<Account>
    where
        F: FnMut(Option<Account>) -> Option<Account>,
    {
        let opt_prev = self.update_opt_value(Key::Account(addr), |opt_value| {
            let opt_account = opt_value.map(|value| match value {
                Value::Account(acc) => acc,
                _ => panic!("Expected account, got {:?}", value),
            });
            f(opt_account).map(|acc| Value::Account(acc))
        });

        opt_prev.map(|prev| match prev {
            Value::Account(acc) => acc,
            _ => panic!("Expected account, got {:?}", prev),
        })
    }

    fn get_account(&self, addr: H160) -> Option<Account> {
        self.update_opt_account(addr, identity)
    }

    /// Get the given account's balance, or 0 if no such account exists.
    fn get_balance(&self, addr: H160) -> U256 {
        self.get_account(addr)
            .map_or(U256::zero(), |acc| acc.balance)
    }

    /// Get the given account's code, or an empty result if no such account exists.
    fn get_code(&self, addr: H160) -> Vec<F> {
        self.get_account(addr).map_or(vec![], |acc| acc.code)
    }

    /// Get a portion of the given account's code, or an empty result if no such account exists.
    fn get_code_range(&self, addr: H160, offset: usize, len: usize) -> Vec<F> {
        let code = self.get_code(addr);
        if code.len() < offset {
            return vec![];
        }
        code[offset..(offset + len).min(code.len())].to_vec()
    }

    /// Get the given account's code size, or 0 if no such account exists.
    fn get_code_size(&self, addr: H160) -> usize {
        self.get_code(addr).len()
    }

    /// Add ETH to the given address.
    fn add_balance(&mut self, addr: H160, value: U256) {
        if value.is_zero() {
            return;
        }

        self.update_opt_account(addr, |opt_acc| {
            let prev_acc = opt_acc.unwrap_or_default();
            Some(prev_acc.with_value_added(value))
        });
    }

    /// Subtract ETH from the given address.
    fn sub_balance(&mut self, addr: H160, value: U256) -> anyhow::Result<()> {
        if value.is_zero() {
            return Ok(());
        }

        let try_update = |opt_acc: &Option<Account>| -> anyhow::Result<Option<Account>> {
            opt_acc.unwrap_or_default().with_value_subtracted(value).map(|acc| Some(acc))
        };

        self.update_opt_account(addr, |opt_acc| {
            try_update(&opt_acc).unwrap_or(opt_acc)
        });
    }

    /// Read a word from the given account's storage. Defaults to zero.
    fn read_storage(&self, addr: H160, key: H256) -> H256;

    /// Write a word to the given account's storage.
    fn write_storage(&mut self, addr: H160, key: H256, value: H256);

    /// Create a new smart contract. The address must be generated in accordance with the `ADDR`
    /// function in the yellowpaper; it is the callers responsibility to ensure that.
    fn create(&mut self, addr: H160, endowment: U256, code: Vec<u64>) -> anyhow::Result<()>;
}
