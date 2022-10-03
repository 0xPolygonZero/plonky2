use ethereum_types::U256;

pub(crate) fn all_rlp_prover_inputs_reversed(signed_txns: &[Vec<u8>]) -> Vec<U256> {
    let mut inputs = all_rlp_prover_inputs(signed_txns);
    inputs.reverse();
    inputs
}

fn all_rlp_prover_inputs(signed_txns: &[Vec<u8>]) -> Vec<U256> {
    let mut prover_inputs = vec![];
    for txn in signed_txns {
        prover_inputs.push(txn.len().into());
        for &byte in txn {
            prover_inputs.push(byte.into());
        }
    }
    prover_inputs
}
