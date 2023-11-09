use ethereum_types::U256;

pub(crate) fn all_rlp_prover_inputs_reversed(signed_txn: &[u8]) -> Vec<U256> {
    let mut inputs = all_rlp_prover_inputs(signed_txn);
    inputs.reverse();
    inputs
}

fn all_rlp_prover_inputs(signed_txn: &[u8]) -> Vec<U256> {
    let mut prover_inputs = vec![];
    prover_inputs.push(signed_txn.len().into());
    for &byte in signed_txn {
        prover_inputs.push(byte.into());
    }
    prover_inputs
}
