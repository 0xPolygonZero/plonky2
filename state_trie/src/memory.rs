// use std::marker::PhantomData;
//
// use anyhow::{anyhow, bail};
// use plonky2::field::goldilocks_field::GoldilocksField;
// use plonky2::hash::merkle_proofs::MerkleProof;
// use plonky2::hash::poseidon::PoseidonHash;
// use plonky2::plonk::config::Hasher;
// use primitive_types::{H160, H256, U256};
//
// use crate::account::Account;
// use crate::util::get_bit;
// use crate::StateTrie;
//
// type F = GoldilocksField;
// type MyHasher = PoseidonHash;
// type MyHash = <MyHasher as Hasher<F>>::Hash;
//
// const ZERO_HASH: MyHash = MyHash::ZERO;
//
// pub struct MemoryMerkleTree {
//     root: Node,
//     _phantom: PhantomData<MyHasher>,
// }
//
// impl MemoryMerkleTree {
//     pub fn new() -> Self {
//         Self {
//             root: Node::empty(),
//             _phantom: PhantomData,
//         }
//     }
//
//     pub fn root(&self) -> MyHash {
//         self.root.digest()
//     }
//
//     fn get(&self, key: &H256) -> Option<(&V, MerkleProof<F, MyHasher>)> {
//         self.root.get(key, 0)
//     }
// }
//
// enum Node {
//     Branch(MyHash, Box<Node>, Box<Node>),
//     Singleton(H256, V),
//     Empty,
// }
//
// impl Node {
//     fn empty() -> Self {
//         Node::Empty(PhantomData)
//     }
//
//     fn digest(&self) -> MyHash {
//         match self {
//             Node::Branch(digest, _, _) => *digest,
//             Node::Singleton(k, v) => todo!(),
//             Node::Empty(_) => ZERO_HASH,
//         }
//     }
//
//     fn get(&self, key: &H256, depth: usize) -> Option<(&V, MerkleProof<F, MyHasher>)> {
//         match self {
//             Node::Branch(_digest, left, right) => {
//                 let bit = get_bit(key, depth);
//                 let (selected, other) = if bit { (right, left) } else { (left, right) };
//                 let mut result = selected.get(key, depth + 1);
//                 for (_v, proof) in result.iter_mut() {
//                     proof.siblings[depth] = other.digest();
//                 }
//                 result
//             }
//             Node::Singleton(k, v) => {
//                 if k == key {
//                     // TODO: Need to exclude first depth bits from comparison.
//                     let siblings = vec![ZERO_HASH; depth];
//                     Some((v, MerkleProof { siblings }))
//                 } else {
//                     None
//                 }
//             }
//             Node::Empty(_) => None,
//         }
//     }
// }
//
// impl StateTrie for MemoryMerkleTree {
//     fn get_account(&self, addr: H160) -> Option<Account> {
//         todo!()
//     }
//
//     fn update_account<F>(&self, addr: H160) -> Option<Account>
//     where
//         F: FnMut(Option<Account>) -> Option<Account>,
//     {
//         todo!()
//     }
//
//     fn add_balance(&mut self, addr: H160, value: U256) {
//         let acc = self.accounts.entry(addr).or_default();
//         acc.balance += value;
//     }
//
//     fn sub_balance(&mut self, addr: H160, value: U256) -> anyhow::Result<()> {
//         let acc = self
//             .accounts
//             .get_mut(&addr)
//             .ok_or(anyhow!("No such account"))?;
//         if acc.balance >= value {
//             acc.balance -= value;
//             Ok(())
//         } else {
//             bail!("Insufficient balance");
//         }
//     }
//
//     fn read_storage(&self, addr: H160, key: H256) -> H256 {
//         let acc = self.accounts.get(&addr).expect("No such address");
//         acc.storage.get(&key).copied().unwrap_or(H256::zero())
//     }
//
//     fn write_storage(&mut self, addr: H160, key: H256, value: H256) {
//         let acc = self.accounts.get_mut(&addr).expect("No such address");
//         acc.storage.insert(key, value);
//     }
//
//     fn create(&mut self, addr: H160, endowment: U256, code: Vec<u64>) -> anyhow::Result<()> {
//         let acc = self.accounts.entry(addr).or_default();
//         if acc.nonce != 0 {
//             bail!("Can't create as there is already a nonzero nonce for this address");
//         }
//         if !acc.code.is_empty() {
//             bail!("Can't create as there is already nonempty code for this address");
//         }
//         acc.code = code;
//         acc.balance += endowment;
//         acc.nonce += 1;
//         Ok(())
//     }
// }
