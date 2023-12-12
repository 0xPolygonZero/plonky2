# Provable Stateless ZK-EVM

Included here is an implementation of a stateless, recursive ZK-EVM client implemented using Plonky2. It currently supports the full Merkle-Patricia Trie and has all Shanghai opcodes implemented.

## Performance 

This implementation is able to provide transaction level proofs which are then recursively aggregated into a block proof. This means that proofs for a block can be efficiently distributed across a cluster of computers. As these proofs use Plonky2 they are CPU and Memory bound. The ability to scale horizontally across transactions increases the total performance of the system dramatically. End-to-end workflows are currently in progress to support this proving mode against live evm networks.

Furthermore the implementation itself is highly optimized to provide fast proving times on generally available cloud instances and does not require GPUs or special hardware.

## Ethereum Compatibility

The aim of this module is to initially provide full ethereum compatibility. Today, all [EVM tests](https://github.com/0xPolygonZero/evm-tests) for the Shanghai hardfork are implemented. Work is progressing on supporting the upcoming [Cancun](https://github.com/0xPolygonZero/plonky2/labels/cancun) EVM changes. Furthermore, this prover uses the full ethereum state tree and hashing modes.

## Audits

Audits for the ZK-EVM will begin on November 27th, 2023. See the [Audit RC1 Milestone](https://github.com/0xPolygonZero/plonky2/milestone/2?closed=1). This README will be updated with the proper branches and hashes when the audit has commenced.

## Documentation / Specification

The current specification is located in the [/spec](/spec) directory, with the most currently up-to-date PDF [available here](https://github.com/0xPolygonZero/plonky2/blob/main/evm/spec/zkevm.pdf). Further documentation will be made over the coming months.

---
Copyright (C) 2023 PT Services DMCC
