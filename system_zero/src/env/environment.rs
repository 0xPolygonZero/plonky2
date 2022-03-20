use primitive_types::{H160, H256, U256};

/// An interface to the blockchain environment, for reading and writing various chain data.
pub trait Environment {
    /// Get the given account's balance, or 0 if no such account exists.
    fn get_balance(&self, addr: H160) -> U256;

    /// Add ETH to the given address.
    fn add_balance(&mut self, addr: H160, value: U256);

    /// Get the given account's code, or an empty result if no such account exists.
    fn get_code(&self, addr: H160) -> Vec<u64>;

    /// Get the given account's code size, or 0 if no such account exists.
    fn get_code_size(&self, addr: H160) -> usize;

    /// Get a portion of the given account's code, or an empty result if no such account exists.
    fn get_code_range(&self, addr: H160, offset: usize, len: usize) -> Vec<u64>;

    /// Read a word from the given account's storage. Panics if no such account exists.
    fn read_storage(&self, addr: H160, key: H256) -> H256;

    /// Write a word to the given account's storage. Panics if no such account exists.
    fn write_storage(&mut self, addr: H160, key: H256, value: H256);

    /// Create a new smart contract.
    fn create(&mut self, addr: H160, endowment: U256, code: Vec<u64>);
}
