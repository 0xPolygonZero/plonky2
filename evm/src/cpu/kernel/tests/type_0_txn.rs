use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;
use hex_literal::hex;
use NormalizedTxnField::*;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::rlp::set_rlp_memory;
use crate::cpu::kernel::txn_fields::NormalizedTxnField;

#[test]
fn process_type_0_txn() -> Result<()> {
    let process_type_0_txn = KERNEL.global_labels["process_type_0_txn"];
    let process_normalized_txn = KERNEL.global_labels["process_normalized_txn"];

    let mut interpreter = Interpreter::new_with_kernel(process_type_0_txn, vec![]);

    // When we reach process_normalized_txn, we're done with parsing and normalizing.
    // Processing normalized transactions is outside the scope of this test.
    interpreter.halt_offsets.push(process_normalized_txn);

    // Generated with py-evm:
    // unsigned_txn = chain.create_unsigned_transaction(
    //     nonce=5,
    //     gas_price=10,
    //     gas=22_000,
    //     to=constants.ZERO_ADDRESS,
    //     value=100,
    //     data=b'\x42\x42',
    // )
    // my_txn = unsigned_txn.as_signed_transaction(my_sk)
    // rlp.encode(my_txn)
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
