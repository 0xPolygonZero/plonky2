# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - TBD

### Changed
- Remove obsolete function `ceil_div_usize` ([#1574](https://github.com/0xPolygonZero/plonky2/pull/1574))
- Update unconstrained_stark.rs ([#1578](https://github.com/0xPolygonZero/plonky2/pull/1578))
- Remove restriction to binary-only multiplicities ([#1577](https://github.com/0xPolygonZero/plonky2/pull/1577))
- Fix `verify_cross_table_lookups` with no `ctl_extra_looking_sums` ([#1584](https://github.com/0xPolygonZero/plonky2/pull/1584))
- update 2-adic generator to `0x64fdd1a46201e246` ([#1579](https://github.com/0xPolygonZero/plonky2/pull/1579))
- Changes to prepare for dummy segment removal in zk_evm's continuations ([#1587](https://github.com/0xPolygonZero/plonky2/pull/1587))
- fix: remove clippy unexpected_cfgs warning ([#1588](https://github.com/0xPolygonZero/plonky2/pull/1588))
- doc+fix: `clippy::doc-lazy-continuation` ([#1594](https://github.com/0xPolygonZero/plonky2/pull/1594))
- change `set_stark_proof_target`'s witness to `WitnessWrite` ([#1592](https://github.com/0xPolygonZero/plonky2/pull/1592))
- Fix CTL generation of last row ([#1585](https://github.com/0xPolygonZero/plonky2/pull/1585))
- Allow multiple extra_looking_sums for the same looked table ([#1591](https://github.com/0xPolygonZero/plonky2/pull/1591))
- Clarify zk usage with starky ([#1596](https://github.com/0xPolygonZero/plonky2/pull/1596))
- Add row index to constraint failure message ([#1598](https://github.com/0xPolygonZero/plonky2/pull/1598))

## [0.2.3] - 2024-04-16

### Changed
- Code refactoring ([#1558](https://github.com/0xPolygonZero/plonky2/pull/1558))
- Simplify types: remove option from CTL filters ([#1567](https://github.com/0xPolygonZero/plonky2/pull/1567))
- Add stdarch_x86_avx512 feature ([#1566](https://github.com/0xPolygonZero/plonky2/pull/1566))

## [0.2.2] - 2024-03-21

### Changed
- Fix CTLs with exactly two looking tables ([#1555](https://github.com/0xPolygonZero/plonky2/pull/1555))
- Make Starks without constraints provable ([#1552](https://github.com/0xPolygonZero/plonky2/pull/1552))

## [0.2.1] - 2024-03-01 (`starky` crate only)

### Changed
- Always compile cross_table_lookups::debug_utils ([#1540](https://github.com/0xPolygonZero/plonky2/pull/1540))

## [0.2.0] - 2024-02-20
- Initial CHANGELOG tracking.
