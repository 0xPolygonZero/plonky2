use std::collections::HashMap;

use primitive_types::{H160, H256, U256};

use crate::env::environment::Environment;

pub struct MockEnvironment {
    accounts: HashMap<H160, MockAccount>,
}

impl MockEnvironment {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }
}

impl Environment for MockEnvironment {
    fn get_balance(&self, addr: H160) -> U256 {
        self.accounts
            .get(&addr)
            .map(|acc| acc.balance)
            .unwrap_or(U256::zero())
    }

    fn add_balance(&mut self, addr: H160, value: U256) {
        let acc = self.accounts.entry(addr).or_default();
        acc.balance += value;
    }

    fn get_code(&self, addr: H160) -> Vec<u64> {
        self.accounts
            .get(&addr)
            .map(|acc| acc.code.clone())
            .unwrap_or_else(Vec::new)
    }

    fn get_code_size(&self, addr: H160) -> usize {
        self.get_code(addr).len()
    }

    fn get_code_range(&self, addr: H160, offset: usize, len: usize) -> Vec<u64> {
        let code = self.get_code(addr);
        if code.len() < offset {
            return vec![];
        }
        code[offset..(offset + len).min(code.len())].to_vec()
    }

    fn read_storage(&self, addr: H160, key: H256) -> H256 {
        let acc = self.accounts.get(&addr).expect("No such address");
        acc.storage.get(&key).copied().unwrap_or(H256::zero())
    }

    fn write_storage(&mut self, addr: H160, key: H256, value: H256) {
        let acc = self.accounts.get_mut(&addr).expect("No such address");
        acc.storage.insert(key, value);
    }

    fn create(&mut self, addr: H160, endowment: U256, code: Vec<u64>) {
        let acc = self.accounts.entry(addr).or_default();
        acc.code = code;
        acc.balance += endowment;
    }
}

struct MockAccount {
    balance: U256,
    code: Vec<u64>,
    storage: HashMap<H256, H256>,
}

impl Default for MockAccount {
    fn default() -> Self {
        Self {
            balance: U256::zero(),
            code: vec![],
            storage: HashMap::new(),
        }
    }
}
