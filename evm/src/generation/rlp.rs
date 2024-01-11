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
    let mut chunks = signed_txn.chunks_exact(32);
    while let Some(bytes) = chunks.next() {
        prover_inputs.push(U256::from_big_endian(bytes));
    }
    let mut last_chunk = chunks.remainder().to_vec();
    if last_chunk.len() > 0 {
        last_chunk.extend_from_slice(&vec![0u8; 32 - last_chunk.len()]);
        prover_inputs.push(U256::from_big_endian(&last_chunk));
    }
    log::debug!(
        "rlp_prover_inputs = {:?}",
        prover_inputs
            .iter()
            .map(|x| {
                let mut bytes = [0u8; 32];
                x.to_big_endian(&mut bytes);
                bytes.to_vec()
            })
            .flatten()
            .collect::<Vec<u8>>()
    );
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
