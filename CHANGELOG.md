# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2024-11-25
- Add constraints binding ([#1679](https://github.com/0xPolygonZero/plonky2/pull/1679))
- Observe config parameters in challenger ([#1678](https://github.com/0xPolygonZero/plonky2/pull/1678))
- chore: update broken links `LICENSE`([#1675](https://github.com/0xPolygonZero/plonky2/pull/1675))
- Fix latest clippy ([#1676](https://github.com/0xPolygonZero/plonky2/pull/1676))
- Fix documentation rendering ([#1671](https://github.com/0xPolygonZero/plonky2/pull/1671))
- Misc updates ([#1663](https://github.com/0xPolygonZero/plonky2/pull/1663))
- Fix clippy ([#1662](https://github.com/0xPolygonZero/plonky2/pull/1662))
- Fix padding for LookupTableGate ([#1661](https://github.com/0xPolygonZero/plonky2/pull/1661))
- Fix padding in LookupTableGate ([#1656](https://github.com/0xPolygonZero/plonky2/pull/1656))
- (fix-lookup) fix: correct visibility in gate_serialization macros ([#1650](https://github.com/0xPolygonZero/plonky2/pull/1650))
- fix: use u64 in BaseSplitGenerator ([#1647](https://github.com/0xPolygonZero/plonky2/pull/1647))
- add serialization and deserialization for BytesHash ([#1645](https://github.com/0xPolygonZero/plonky2/pull/1645))
- (clippy) fix: changed conditioning for timing functionality in circuit_builder ([#1640](https://github.com/0xPolygonZero/plonky2/pull/1640))

## [1.0.0] - 2024-11-25

### Changed
- Unified Recursion Circuit for Multi-Degree Starky Proof Verification ([#1635](https://github.com/0xPolygonZero/plonky2/pull/1635))
- Fix `DummyProofGenerator` serialization ([#1634](https://github.com/0xPolygonZero/plonky2/pull/1634))
- Refactor CTL Handling ([#1629](https://github.com/0xPolygonZero/plonky2/pull/1629))
- Added serialize and deserialize to starky proofs ([#1630](https://github.com/0xPolygonZero/plonky2/pull/1630))
- changed to web-time in circuit_builder ([#1624](https://github.com/0xPolygonZero/plonky2/pull/1624))
- Fix example and documentation rendering ([#1614](https://github.com/0xPolygonZero/plonky2/pull/1614))
- Add `connect_array` convenience method in `CircuitBuilder` ([#1620](https://github.com/0xPolygonZero/plonky2/pull/1620))
- chore: remove compressed StarkProof variant ([#1618](https://github.com/0xPolygonZero/plonky2/pull/1618))
- Do not panic on `wire set twice` or `generator not run` issues ([#1611](https://github.com/0xPolygonZero/plonky2/pull/1611))
- Add Support for Batch STARKs with Proving, Verification, and Recursion ([#1600](https://github.com/0xPolygonZero/plonky2/pull/1600))
- chore: fix clippy ([#1609](https://github.com/0xPolygonZero/plonky2/pull/1609))
- fix(starky): observe public inputs ([#1607](https://github.com/0xPolygonZero/plonky2/pull/1607))
- ci: add PR check job ([#1604](https://github.com/0xPolygonZero/plonky2/pull/1604))
- Add `Field::shifted_powers` and some iterator niceties ([#1599](https://github.com/0xPolygonZero/plonky2/pull/1599))
- fix(field): Re-enable `alloc` for tests ([#1601](https://github.com/0xPolygonZero/plonky2/pull/1601))
- Add row index to constraint failure message ([#1598](https://github.com/0xPolygonZero/plonky2/pull/1598))
- doc: clarify that `zk` is disabled with `starky` ([#1596](https://github.com/0xPolygonZero/plonky2/pull/1596))
- Allow multiple `extra_looking_sums` for the same looked table ([#1591](https://github.com/0xPolygonZero/plonky2/pull/1591))
- Fix CTL generation of last row ([#1585](https://github.com/0xPolygonZero/plonky2/pull/1585))
- change `set_stark_proof_target`'s witness to `WitnessWrite` ([#1592](https://github.com/0xPolygonZero/plonky2/pull/1592))
- doc+fix: `clippy::doc-lazy-continuation` ([#1594](https://github.com/0xPolygonZero/plonky2/pull/1594))
- fix: remove clippy unexpected_cfgs warning ([#1588](https://github.com/0xPolygonZero/plonky2/pull/1588))
- Changes to prepare for dummy segment removal in zk_evm's continuations ([#1587](https://github.com/0xPolygonZero/plonky2/pull/1587))
- update 2-adic generator to `0x64fdd1a46201e246` ([#1579](https://github.com/0xPolygonZero/plonky2/pull/1579))
- Fix `verify_cross_table_lookups` with no `ctl_extra_looking_sums` ([#1584](https://github.com/0xPolygonZero/plonky2/pull/1584))
- Remove restriction to binary-only multiplicities ([#1577](https://github.com/0xPolygonZero/plonky2/pull/1577))
- Remove obsolete function `ceil_div_usize` ([#1574](https://github.com/0xPolygonZero/plonky2/pull/1574))


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
