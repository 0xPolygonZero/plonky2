use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H160, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::KeccakGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::generation::mpt::{AccountRlp, LegacyReceiptRlp, LogRlp};
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;

type F = GoldilocksField;
const D: usize = 2;
type C = KeccakGoldilocksConfig;

/// Test a simple ERC20 transfer.
/// Used the following Solidity code:
/// ```solidity
/// pragma solidity ^0.8.13;
/// import "../lib/openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
/// contract Token is ERC20 {
///     constructor() ERC20("Token", "TKN") {
///         _mint(msg.sender, 1_000_000 ether);
///     }
/// }
/// contract Giver {
///     Token public token;
///     constructor(address _token) {
///         token = Token(_token);
///     }
///     function send(uint256 amount) public {
///         token.transfer(0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326, amount);
///     }
/// }
/// ```
#[test]
fn test_erc20() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    let sender = hex!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
    let giver = hex!("e7f1725E7734CE288F8367e1Bb143E90bb3F0512");
    let token = hex!("5FbDB2315678afecb367f032d93F642f64180aa3");

    let sender_state_key = keccak(sender);
    let giver_state_key = keccak(giver);
    let token_state_key = keccak(token);

    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let giver_nibbles = Nibbles::from_bytes_be(giver_state_key.as_bytes()).unwrap();
    let token_nibbles = Nibbles::from_bytes_be(token_state_key.as_bytes()).unwrap();

    let mut state_trie_before = HashedPartialTrie::from(Node::Empty);
    state_trie_before.insert(sender_nibbles, rlp::encode(&sender_account()).to_vec());
    state_trie_before.insert(giver_nibbles, rlp::encode(&giver_account()).to_vec());
    state_trie_before.insert(token_nibbles, rlp::encode(&token_account()).to_vec());

    let storage_tries = vec![
        (giver_state_key, giver_storage()),
        (token_state_key, token_storage()),
    ];

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: HashedPartialTrie::from(Node::Empty),
        receipts_trie: HashedPartialTrie::from(Node::Empty),
        storage_tries,
    };

    let txn = signed_tx();

    let gas_used = 56_499.into();
    let bloom = bloom();
    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_random: H256::from_uint(&0x020000.into()),
        block_gaslimit: 0xff112233u32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: gas_used,
        block_bloom: bloom,
    };

    let contract_code = [giver_bytecode(), token_bytecode(), vec![]]
        .map(|v| (keccak(v.clone()), v))
        .into();

    let expected_state_trie_after: HashedPartialTrie = {
        let mut state_trie_after = HashedPartialTrie::from(Node::Empty);
        let sender_account = sender_account();
        let sender_account_after = AccountRlp {
            nonce: sender_account.nonce + 1,
            balance: sender_account.balance - gas_used * 0xa,
            ..sender_account
        };
        state_trie_after.insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec());
        state_trie_after.insert(giver_nibbles, rlp::encode(&giver_account()).to_vec());
        let token_account_after = AccountRlp {
            storage_root: token_storage_after().hash(),
            ..token_account()
        };
        state_trie_after.insert(token_nibbles, rlp::encode(&token_account_after).to_vec());

        state_trie_after
    };

    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used,
        bloom: bloom_bytes().to_vec().into(),
        logs: vec![LogRlp {
            address: H160::from_str("0x5fbdb2315678afecb367f032d93f642f64180aa3").unwrap(),
            topics: vec![
                H256::from_str(
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                )
                .unwrap(),
                H256::from_str(
                    "0x000000000000000000000000e7f1725e7734ce288f8367e1bb143e90bb3f0512",
                )
                .unwrap(),
                H256::from_str(
                    "0x0000000000000000000000001f9090aae28b8a3dceadf281b0f12828e676c326",
                )
                .unwrap(),
            ],
            data: hex!("0000000000000000000000000000000000000000000000056bc75e2d63100000")
                .to_vec()
                .into(),
        }],
    };
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
    receipts_trie.insert(Nibbles::from_str("0x80").unwrap(), receipt_0.encode(2));
    let transactions_trie: HashedPartialTrie = Node::Leaf {
        nibbles: Nibbles::from_str("0x80").unwrap(),
        value: txn.to_vec(),
    }
    .into();

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txn: Some(txn.to_vec()),
        withdrawals: vec![],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        checkpoint_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: gas_used,
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing, None)?;
    timing.filter(Duration::from_millis(100)).print();

    verify_proof(&all_stark, proof, &config)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}

fn giver_bytecode() -> Vec<u8> {
    hex!("608060405234801561001057600080fd5b50600436106100365760003560e01c8063a52c101e1461003b578063fc0c546a14610050575b600080fd5b61004e61004936600461010c565b61007f565b005b600054610063906001600160a01b031681565b6040516001600160a01b03909116815260200160405180910390f35b60005460405163a9059cbb60e01b8152731f9090aae28b8a3dceadf281b0f12828e676c3266004820152602481018390526001600160a01b039091169063a9059cbb906044016020604051808303816000875af11580156100e4573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906101089190610125565b5050565b60006020828403121561011e57600080fd5b5035919050565b60006020828403121561013757600080fd5b8151801515811461014757600080fd5b939250505056fea264697066735822122050741efdbac11eb0bbb776ce3ac6004e596b7d7559658a12506164388c371cfd64736f6c63430008140033").into()
}

fn token_bytecode() -> Vec<u8> {
    hex!("608060405234801561001057600080fd5b50600436106100935760003560e01c8063313ce56711610066578063313ce567146100fe57806370a082311461010d57806395d89b4114610136578063a9059cbb1461013e578063dd62ed3e1461015157600080fd5b806306fdde0314610098578063095ea7b3146100b657806318160ddd146100d957806323b872dd146100eb575b600080fd5b6100a061018a565b6040516100ad919061056a565b60405180910390f35b6100c96100c43660046105d4565b61021c565b60405190151581526020016100ad565b6002545b6040519081526020016100ad565b6100c96100f93660046105fe565b610236565b604051601281526020016100ad565b6100dd61011b36600461063a565b6001600160a01b031660009081526020819052604090205490565b6100a061025a565b6100c961014c3660046105d4565b610269565b6100dd61015f36600461065c565b6001600160a01b03918216600090815260016020908152604080832093909416825291909152205490565b6060600380546101999061068f565b80601f01602080910402602001604051908101604052809291908181526020018280546101c59061068f565b80156102125780601f106101e757610100808354040283529160200191610212565b820191906000526020600020905b8154815290600101906020018083116101f557829003601f168201915b5050505050905090565b60003361022a818585610277565b60019150505b92915050565b600033610244858285610289565b61024f85858561030c565b506001949350505050565b6060600480546101999061068f565b60003361022a81858561030c565b610284838383600161036b565b505050565b6001600160a01b03838116600090815260016020908152604080832093861683529290522054600019811461030657818110156102f757604051637dc7a0d960e11b81526001600160a01b038416600482015260248101829052604481018390526064015b60405180910390fd5b6103068484848403600061036b565b50505050565b6001600160a01b03831661033657604051634b637e8f60e11b8152600060048201526024016102ee565b6001600160a01b0382166103605760405163ec442f0560e01b8152600060048201526024016102ee565b610284838383610440565b6001600160a01b0384166103955760405163e602df0560e01b8152600060048201526024016102ee565b6001600160a01b0383166103bf57604051634a1406b160e11b8152600060048201526024016102ee565b6001600160a01b038085166000908152600160209081526040808320938716835292905220829055801561030657826001600160a01b0316846001600160a01b03167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b9258460405161043291815260200190565b60405180910390a350505050565b6001600160a01b03831661046b57806002600082825461046091906106c9565b909155506104dd9050565b6001600160a01b038316600090815260208190526040902054818110156104be5760405163391434e360e21b81526001600160a01b038516600482015260248101829052604481018390526064016102ee565b6001600160a01b03841660009081526020819052604090209082900390555b6001600160a01b0382166104f957600280548290039055610518565b6001600160a01b03821660009081526020819052604090208054820190555b816001600160a01b0316836001600160a01b03167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef8360405161055d91815260200190565b60405180910390a3505050565b600060208083528351808285015260005b818110156105975785810183015185820160400152820161057b565b506000604082860101526040601f19601f8301168501019250505092915050565b80356001600160a01b03811681146105cf57600080fd5b919050565b600080604083850312156105e757600080fd5b6105f0836105b8565b946020939093013593505050565b60008060006060848603121561061357600080fd5b61061c846105b8565b925061062a602085016105b8565b9150604084013590509250925092565b60006020828403121561064c57600080fd5b610655826105b8565b9392505050565b6000806040838503121561066f57600080fd5b610678836105b8565b9150610686602084016105b8565b90509250929050565b600181811c908216806106a357607f821691505b6020821081036106c357634e487b7160e01b600052602260045260246000fd5b50919050565b8082018082111561023057634e487b7160e01b600052601160045260246000fdfea2646970667358221220266a323ae4a816f6c6342a5be431fedcc0d45c44b02ea75f5474eb450b5d45b364736f6c63430008140033").into()
}

fn insert_storage(trie: &mut HashedPartialTrie, slot: U256, value: U256) {
    let mut bytes = [0; 32];
    slot.to_big_endian(&mut bytes);
    let key = keccak(bytes);
    let nibbles = Nibbles::from_bytes_be(key.as_bytes()).unwrap();
    let r = rlp::encode(&value);
    let r = r.freeze().to_vec();
    trie.insert(nibbles, r);
}

fn sd2u(s: &str) -> U256 {
    U256::from_dec_str(s).unwrap()
}

fn giver_storage() -> HashedPartialTrie {
    let mut trie = HashedPartialTrie::from(Node::Empty);
    insert_storage(
        &mut trie,
        U256::zero(),
        sd2u("546584486846459126461364135121053344201067465379"),
    );
    trie
}

fn token_storage() -> HashedPartialTrie {
    let mut trie = HashedPartialTrie::from(Node::Empty);
    insert_storage(
        &mut trie,
        sd2u("82183438603287090451672504949863617512989139203883434767553028632841710582583"),
        sd2u("1000000000000000000000"),
    );
    trie
}

fn token_storage_after() -> HashedPartialTrie {
    let mut trie = HashedPartialTrie::from(Node::Empty);
    insert_storage(
        &mut trie,
        sd2u("82183438603287090451672504949863617512989139203883434767553028632841710582583"),
        sd2u("900000000000000000000"),
    );
    insert_storage(
        &mut trie,
        sd2u("53006154680716014998529145169423020330606407246856709517064848190396281160729"),
        sd2u("100000000000000000000"),
    );
    trie
}

fn giver_account() -> AccountRlp {
    AccountRlp {
        nonce: 1.into(),
        balance: 0.into(),
        storage_root: giver_storage().hash(),
        code_hash: keccak(giver_bytecode()),
    }
}

fn token_account() -> AccountRlp {
    AccountRlp {
        nonce: 1.into(),
        balance: 0.into(),
        storage_root: token_storage().hash(),
        code_hash: keccak(token_bytecode()),
    }
}

fn sender_account() -> AccountRlp {
    AccountRlp {
        nonce: 0.into(),
        balance: sd2u("10000000000000000000000"),
        storage_root: Default::default(),
        code_hash: keccak([]),
    }
}

fn signed_tx() -> Vec<u8> {
    hex!("02f88701800a0a830142c594e7f1725e7734ce288f8367e1bb143e90bb3f051280a4a52c101e0000000000000000000000000000000000000000000000056bc75e2d63100000c001a0303f5591159d7ea303faecb1c8bd8624b55732f769de28b111190dfb9a7c5234a019d5d6d38938dc1c63acbe106cf361672def773ace4ca587860117d057326627").into()
}

fn bloom_bytes() -> [u8; 256] {
    hex!("00000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000008000000000000000000000000000000000040000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000002000000000000000000000000000000000000000000000042000000000000000000000000000000000000000000020000000000080000000000000000000000000000000000000000000000000000000000000000")
}

fn bloom() -> [U256; 8] {
    let bloom = bloom_bytes()
        .chunks_exact(32)
        .map(U256::from_big_endian)
        .collect::<Vec<_>>();
    bloom.try_into().unwrap()
}
