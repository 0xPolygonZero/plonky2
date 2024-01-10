use ethereum_types::U256;

pub(crate) fn all_rlp_prover_inputs_reversed(signed_txn: &[u8]) -> Vec<U256> {
    let mut inputs = all_rlp_prover_inputs(signed_txn);
    inputs.reverse();
    inputs
}

pub(crate) fn all_rlp_prover_inputs_reversed_old(signed_txn: &[u8]) -> Vec<U256> {
    let mut inputs = all_rlp_prover_inputs_old(signed_txn);
    inputs.reverse();
    inputs
}

fn all_rlp_prover_inputs(signed_txn: &[u8]) -> Vec<U256> {
    let mut prover_inputs = vec![];
    prover_inputs.push(signed_txn.len().into());
    for bytes in signed_txn.chunks(32) {
        prover_inputs.push(U256::from_big_endian(bytes));
    }
    log::debug!("rlp_prover_inputs = {:?}", prover_inputs);
    prover_inputs
}

fn all_rlp_prover_inputs_old(signed_txn: &[u8]) -> Vec<U256> {
    let mut prover_inputs = vec![];
    prover_inputs.push(signed_txn.len().into());
    for &byte in signed_txn {
        prover_inputs.push(byte.into());
    }
    log::debug!("rlp_prover_inputs_old = {:?}", prover_inputs);
    prover_inputs
}
