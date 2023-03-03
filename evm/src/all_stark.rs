use std::iter;

use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::arithmetic::arithmetic_stark;
use crate::arithmetic::arithmetic_stark::ArithmeticStark;
use crate::config::StarkConfig;
use crate::cpu::columns::COL_MAP;
use crate::cpu::cpu_stark;
use crate::cpu::cpu_stark::CpuStark;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cross_table_lookup::{Column, CrossTableLookup, TableWithColumns};
use crate::keccak::keccak_stark;
use crate::keccak::keccak_stark::KeccakStark;
use crate::keccak_sponge::columns::KECCAK_RATE_BYTES;
use crate::keccak_sponge::keccak_sponge_stark;
use crate::keccak_sponge::keccak_sponge_stark::KeccakSpongeStark;
use crate::logic;
use crate::logic::LogicStark;
use crate::memory::memory_stark;
use crate::memory::memory_stark::MemoryStark;
use crate::stark::Stark;

#[derive(Clone)]
pub struct AllStark<F: RichField + Extendable<D>, const D: usize> {
    pub arithmetic_stark: ArithmeticStark<F, D>,
    pub cpu_stark: CpuStark<F, D>,
    pub keccak_stark: KeccakStark<F, D>,
    pub keccak_sponge_stark: KeccakSpongeStark<F, D>,
    pub logic_stark: LogicStark<F, D>,
    pub memory_stark: MemoryStark<F, D>,
    pub cross_table_lookups: Vec<CrossTableLookup<F>>,
}

impl<F: RichField + Extendable<D>, const D: usize> Default for AllStark<F, D> {
    fn default() -> Self {
        Self {
            arithmetic_stark: ArithmeticStark::default(),
            cpu_stark: CpuStark::default(),
            keccak_stark: KeccakStark::default(),
            keccak_sponge_stark: KeccakSpongeStark::default(),
            logic_stark: LogicStark::default(),
            memory_stark: MemoryStark::default(),
            cross_table_lookups: all_cross_table_lookups(),
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> AllStark<F, D> {
    pub(crate) fn nums_permutation_zs(&self, config: &StarkConfig) -> [usize; NUM_TABLES] {
        [
            self.arithmetic_stark.num_permutation_batches(config),
            self.cpu_stark.num_permutation_batches(config),
            self.keccak_stark.num_permutation_batches(config),
            self.keccak_sponge_stark.num_permutation_batches(config),
            self.logic_stark.num_permutation_batches(config),
            self.memory_stark.num_permutation_batches(config),
        ]
    }

    pub(crate) fn permutation_batch_sizes(&self) -> [usize; NUM_TABLES] {
        [
            self.arithmetic_stark.permutation_batch_size(),
            self.cpu_stark.permutation_batch_size(),
            self.keccak_stark.permutation_batch_size(),
            self.keccak_sponge_stark.permutation_batch_size(),
            self.logic_stark.permutation_batch_size(),
            self.memory_stark.permutation_batch_size(),
        ]
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Table {
    Arithmetic = 0,
    Cpu = 1,
    Keccak = 2,
    KeccakSponge = 3,
    Logic = 4,
    Memory = 5,
}

pub(crate) const NUM_TABLES: usize = Table::Memory as usize + 1;

impl Table {
    pub(crate) fn all() -> [Self; NUM_TABLES] {
        [
            Self::Arithmetic,
            Self::Cpu,
            Self::Keccak,
            Self::KeccakSponge,
            Self::Logic,
            Self::Memory,
        ]
    }
}

pub(crate) fn all_cross_table_lookups<F: Field>() -> Vec<CrossTableLookup<F>> {
    let mut ctls = vec![
        ctl_arithmetic_add_mul(),
        ctl_arithmetic_sub(),
        ctl_arithmetic_lt(),
        ctl_arithmetic_gt(),
        ctl_arithmetic_modops(),
        ctl_arithmetic_bn254ops(),
        ctl_arithmetic_div(),
        ctl_arithmetic_mod(),
        ctl_keccak_sponge(),
        ctl_keccak(),
        ctl_logic(),
        ctl_memory(),
    ];
    // TODO: Some CTLs temporarily disabled while we get them working.
    disable_ctl(&mut ctls[8]);
    disable_ctl(&mut ctls[11]);
    ctls
}

fn disable_ctl<F: Field>(ctl: &mut CrossTableLookup<F>) {
    for table in &mut ctl.looking_tables {
        table.filter_column = Some(Column::zero());
    }
    ctl.looked_table.filter_column = Some(Column::zero());
}

fn ctl_arithmetic_add_mul<F: Field>() -> CrossTableLookup<F> {
    const ADD_MUL_OPS: [usize; 2] = [COL_MAP.op.add, COL_MAP.op.mul];
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_binops(&ADD_MUL_OPS),
        Some(cpu_stark::ctl_filter_arithmetic(&ADD_MUL_OPS)),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_add_mul(),
        Some(arithmetic_stark::ctl_filter_add_mul()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_arithmetic_sub<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_binops(&[COL_MAP.op.sub]),
        Some(cpu_stark::ctl_filter_arithmetic(&[COL_MAP.op.sub])),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_sub(),
        Some(arithmetic_stark::ctl_filter_sub()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_arithmetic_lt<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_binops(&[COL_MAP.op.lt]),
        Some(cpu_stark::ctl_filter_arithmetic(&[COL_MAP.op.lt])),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_lt(),
        Some(arithmetic_stark::ctl_filter_lt()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_arithmetic_gt<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_binops(&[COL_MAP.op.gt]),
        Some(cpu_stark::ctl_filter_arithmetic(&[COL_MAP.op.gt])),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_gt(),
        Some(arithmetic_stark::ctl_filter_gt()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_arithmetic_modops<F: Field>() -> CrossTableLookup<F> {
    const MOD_OPS: [usize; 3] = [COL_MAP.op.addmod, COL_MAP.op.mulmod, COL_MAP.op.submod];
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_ternops(&MOD_OPS),
        Some(cpu_stark::ctl_filter_arithmetic(&MOD_OPS)),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_modops(),
        Some(arithmetic_stark::ctl_filter_modops()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_arithmetic_bn254ops<F: Field>() -> CrossTableLookup<F> {
    const BN254_OPS: [usize; 3] = [
        COL_MAP.op.addfp254,
        COL_MAP.op.mulfp254,
        COL_MAP.op.subfp254,
    ];
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_binops(&BN254_OPS),
        Some(cpu_stark::ctl_filter_arithmetic(&BN254_OPS)),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_bn254ops(),
        Some(arithmetic_stark::ctl_filter_bn254ops()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_arithmetic_div<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_binops(&[COL_MAP.op.div]),
        Some(cpu_stark::ctl_filter_arithmetic(&[COL_MAP.op.div])),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_div(),
        Some(arithmetic_stark::ctl_filter_div()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_arithmetic_mod<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_arithmetic_binops(&[COL_MAP.op.mod_]),
        Some(cpu_stark::ctl_filter_arithmetic(&[COL_MAP.op.mod_])),
    );
    let arithmetic_looked = TableWithColumns::new(
        Table::Arithmetic,
        arithmetic_stark::ctl_data_mod(),
        Some(arithmetic_stark::ctl_filter_mod()),
    );
    CrossTableLookup::new(vec![cpu_looking], arithmetic_looked)
}

fn ctl_keccak<F: Field>() -> CrossTableLookup<F> {
    let keccak_sponge_looking = TableWithColumns::new(
        Table::KeccakSponge,
        keccak_sponge_stark::ctl_looking_keccak(),
        Some(keccak_sponge_stark::ctl_looking_keccak_filter()),
    );
    let keccak_looked = TableWithColumns::new(
        Table::Keccak,
        keccak_stark::ctl_data(),
        Some(keccak_stark::ctl_filter()),
    );
    CrossTableLookup::new(vec![keccak_sponge_looking], keccak_looked)
}

fn ctl_keccak_sponge<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_keccak_sponge(),
        Some(cpu_stark::ctl_filter_keccak_sponge()),
    );
    let keccak_sponge_looked = TableWithColumns::new(
        Table::KeccakSponge,
        keccak_sponge_stark::ctl_looked_data(),
        Some(keccak_sponge_stark::ctl_looked_filter()),
    );
    CrossTableLookup::new(vec![cpu_looking], keccak_sponge_looked)
}

fn ctl_logic<F: Field>() -> CrossTableLookup<F> {
    let cpu_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_logic(),
        Some(cpu_stark::ctl_filter_logic()),
    );
    let mut all_lookers = vec![cpu_looking];
    for i in 0..keccak_sponge_stark::num_logic_ctls() {
        let keccak_sponge_looking = TableWithColumns::new(
            Table::KeccakSponge,
            keccak_sponge_stark::ctl_looking_logic(i),
            Some(keccak_sponge_stark::ctl_looking_logic_filter()),
        );
        all_lookers.push(keccak_sponge_looking);
    }
    let logic_looked =
        TableWithColumns::new(Table::Logic, logic::ctl_data(), Some(logic::ctl_filter()));
    CrossTableLookup::new(all_lookers, logic_looked)
}

fn ctl_memory<F: Field>() -> CrossTableLookup<F> {
    let cpu_memory_code_read = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_code_memory(),
        Some(cpu_stark::ctl_filter_code_memory()),
    );
    let cpu_memory_gp_ops = (0..NUM_GP_CHANNELS).map(|channel| {
        TableWithColumns::new(
            Table::Cpu,
            cpu_stark::ctl_data_gp_memory(channel),
            Some(cpu_stark::ctl_filter_gp_memory(channel)),
        )
    });
    let keccak_sponge_reads = (0..KECCAK_RATE_BYTES).map(|i| {
        TableWithColumns::new(
            Table::KeccakSponge,
            keccak_sponge_stark::ctl_looking_memory(i),
            Some(keccak_sponge_stark::ctl_looking_memory_filter(i)),
        )
    });
    let all_lookers = iter::once(cpu_memory_code_read)
        .chain(cpu_memory_gp_ops)
        .chain(keccak_sponge_reads)
        .collect();
    let memory_looked = TableWithColumns::new(
        Table::Memory,
        memory_stark::ctl_data(),
        Some(memory_stark::ctl_filter()),
    );
    CrossTableLookup::new(all_lookers, memory_looked)
}
