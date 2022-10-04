use anyhow::Result;
use ethereum_types::U256;
use hex_literal::hex;
use NormalizedTxnField::*;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::txn_fields::NormalizedTxnField;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn process_type_0_txn() -> Result<()> {
    let process_type_0_txn = KERNEL.global_labels["process_type_0_txn"];
    let process_normalized_txn = KERNEL.global_labels["process_normalized_txn"];

    let retaddr = 0xDEADBEEFu32.into();
    let mut interpreter = Interpreter::new_with_kernel(process_type_0_txn, vec![retaddr]);

    // When we reach process_normalized_txn, we're done with parsing and normalizing.
    // Processing normalized transactions is outside the scope of this test.
    interpreter.halt_offsets.push(process_normalized_txn);

    // Generated with py-evm:
    // import eth, eth_keys, eth_utils, rlp
    // genesis_params = { 'difficulty': eth.constants.GENESIS_DIFFICULTY }
    // chain = eth.chains.mainnet.MainnetChain.from_genesis(eth.db.atomic.AtomicDB(), genesis_params, {})
    // unsigned_txn = chain.create_unsigned_transaction(
    //     nonce=5,
    //     gas_price=10,
    //     gas=22_000,
    //     to=eth.constants.ZERO_ADDRESS,
    //     value=100,
    //     data=b'\x42\x42',
    // )
    // sk = eth_keys.keys.PrivateKey(eth_utils.decode_hex('4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318'))
    // signed_txn = unsigned_txn.as_signed_transaction(sk)
    // rlp.encode(signed_txn).hex()
    interpreter.set_rlp_memory(hex!("f861050a8255f0940000000000000000000000000000000000000000648242421ca07c5c61ed975ebd286f6b027b8c504842e50a47d318e1e801719dd744fe93e6c6a01e7b5119b57dd54e175ff2f055c91f3ab1b53eba0b2c184f347cdff0e745aca2").to_vec());

    interpreter.run()?;

    assert_eq!(interpreter.get_txn_field(ChainIdPresent), 0.into());
    assert_eq!(interpreter.get_txn_field(ChainId), 0.into());
    assert_eq!(interpreter.get_txn_field(Nonce), 5.into());
    assert_eq!(interpreter.get_txn_field(MaxPriorityFeePerGas), 10.into());
    assert_eq!(interpreter.get_txn_field(MaxPriorityFeePerGas), 10.into());
    assert_eq!(interpreter.get_txn_field(MaxFeePerGas), 10.into());
    assert_eq!(interpreter.get_txn_field(To), 0.into());
    assert_eq!(interpreter.get_txn_field(Value), 100.into());
    assert_eq!(interpreter.get_txn_field(DataLen), 2.into());
    assert_eq!(interpreter.get_txn_data(), &[0x42.into(), 0x42.into()]);
    assert_eq!(interpreter.get_txn_field(YParity), 1.into());
    assert_eq!(
        interpreter.get_txn_field(R),
        U256::from_big_endian(&hex!(
            "7c5c61ed975ebd286f6b027b8c504842e50a47d318e1e801719dd744fe93e6c6"
        ))
    );
    assert_eq!(
        interpreter.get_txn_field(S),
        U256::from_big_endian(&hex!(
            "1e7b5119b57dd54e175ff2f055c91f3ab1b53eba0b2c184f347cdff0e745aca2"
        ))
    );

    Ok(())
}
