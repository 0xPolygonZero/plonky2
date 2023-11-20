# Recursive Stateless ZK-EVM

Included here is an implementation of a stateless, recursive ZK-EVM client implemented using Plonky2. It currently supports the full Merkle-Patricia Tree and has all Shanghai opcodes implemented.

## Ethereum Compatibility

The aim of this module is to initially provide full ethereum compatibility. Today, all EVM tests for the Shanghai hardfork are implemented. Work is progressing on supporting the upcoming [Cancun](https://github.com/0xPolygonZero/plonky2/labels/cancun) EVM changes. Furthermore, this prover uses the full ethereum state tree and hashing modes.

## Audits

Audits for the ZK-EVM will begin on November 27th, 2023. See the [Audit RC1 Milestone](https://github.com/0xPolygonZero/plonky2/milestone/2?closed=1). This README will be updated with the proper branches and hashes when the audit has commenced.

## Documentation / Specification

The current specification is located in the [/spec](/spec) directory, with the most currently up-to-date PDF [availabe here](https://github.com/0xPolygonZero/plonky2/blob/main/evm/spec/zkevm.pdf). Further documentation will be made over the coming months.

---
Copyright (C) 2023 PT Services DMCC
