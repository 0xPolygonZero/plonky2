use std::iter;

use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::arithmetic::arithmetic_stark;
use crate::arithmetic::arithmetic_stark::ArithmeticStark;
use crate::byte_packing::byte_packing_stark::{self, BytePackingStark};
use crate::config::StarkConfig;
use crate::cpu::cpu_stark;
use crate::cpu::cpu_stark::CpuStark;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cross_table_lookup::{CrossTableLookup, TableWithColumns};
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

/// Structure containing all STARKs and the cross-table lookups.
#[derive(Clone)]
pub struct AllStark<F: RichField + Extendable<D>, const D: usize> {
    pub(crate) arithmetic_stark: ArithmeticStark<F, D>,
    pub(crate) byte_packing_stark: BytePackingStark<F, D>,
    pub(crate) cpu_stark: CpuStark<F, D>,
    pub(crate) keccak_stark: KeccakStark<F, D>,
    pub(crate) keccak_sponge_stark: KeccakSpongeStark<F, D>,
    pub(crate) logic_stark: LogicStark<F, D>,
    pub(crate) memory_stark: MemoryStark<F, D>,
    pub(crate) cross_table_lookups: Vec<CrossTableLookup<F>>,
}

impl<F: RichField + Extendable<D>, const D: usize> Default for AllStark<F, D> {
    /// Returns an `AllStark` containing all the STARKs initialized with default values.
    fn default() -> Self {
        Self {
            arithmetic_stark: ArithmeticStark::default(),
            byte_packing_stark: BytePackingStark::default(),
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
    pub(crate) fn num_lookups_helper_columns(&self, config: &StarkConfig) -> [usize; NUM_TABLES] {
        [
            self.arithmetic_stark.num_lookup_helper_columns(config),
            self.byte_packing_stark.num_lookup_helper_columns(config),
            self.cpu_stark.num_lookup_helper_columns(config),
            self.keccak_stark.num_lookup_helper_columns(config),
            self.keccak_sponge_stark.num_lookup_helper_columns(config),
            self.logic_stark.num_lookup_helper_columns(config),
            self.memory_stark.num_lookup_helper_columns(config),
        ]
    }
}

/// Associates STARK tables with a unique index.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Table {
    Arithmetic = 0,
    BytePacking = 1,
    Cpu = 2,
    Keccak = 3,
    KeccakSponge = 4,
    Logic = 5,
    Memory = 6,
}

/// Number of STARK tables.
pub(crate) const NUM_TABLES: usize = Table::Memory as usize + 1;

impl Table {
    /// Returns all STARK table indices.
    pub(crate) const fn all() -> [Self; NUM_TABLES] {
        [
            Self::Arithmetic,
            Self::BytePacking,
            Self::Cpu,
            Self::Keccak,
            Self::KeccakSponge,
            Self::Logic,
            Self::Memory,
        ]
    }
}

/// Returns all the `CrossTableLookups` used for proving the EVM.
pub(crate) fn all_cross_table_lookups<F: Field>() -> Vec<CrossTableLookup<F>> {
    vec![
        ctl_arithmetic(),
        ctl_byte_packing(),
        ctl_keccak_sponge(),
        ctl_keccak_inputs(),
        ctl_keccak_outputs(),
        ctl_logic(),
        ctl_memory(),
    ]
}

/// `CrossTableLookup` for `ArithmeticStark`, to connect it with the `Cpu` module.
fn ctl_arithmetic<F: Field>() -> CrossTableLookup<F> {
    CrossTableLookup::new(
        vec![cpu_stark::ctl_arithmetic_base_rows()],
        arithmetic_stark::ctl_arithmetic_rows(),
    )
}

/// `CrossTableLookup` for `BytePackingStark`, to connect it with the `Cpu` module.
fn ctl_byte_packing<F: Field>() -> CrossTableLookup<F> {
    let cpu_packing_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_byte_packing(),
        Some(cpu_stark::ctl_filter_byte_packing()),
    );
    let cpu_unpacking_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_byte_unpacking(),
        Some(cpu_stark::ctl_filter_byte_unpacking()),
    );
    let cpu_push_packing_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_byte_packing_push(),
        Some(cpu_stark::ctl_filter_byte_packing_push()),
    );
    let cpu_jumptable_read_looking = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_jumptable_read(),
        Some(cpu_stark::ctl_filter_syscall_exceptions()),
    );
    let byte_packing_looked = TableWithColumns::new(
        Table::BytePacking,
        byte_packing_stark::ctl_looked_data(),
        Some(byte_packing_stark::ctl_looked_filter()),
    );
    CrossTableLookup::new(
        vec![
            cpu_packing_looking,
            cpu_unpacking_looking,
            cpu_push_packing_looking,
            cpu_jumptable_read_looking,
        ],
        byte_packing_looked,
    )
}

/// `CrossTableLookup` for `KeccakStark` inputs, to connect it with the `KeccakSponge` module.
/// `KeccakStarkSponge` looks into `KeccakStark` to give the inputs of the sponge.
/// Its consistency with the 'output' CTL is ensured through a timestamp column on the `KeccakStark` side.
fn ctl_keccak_inputs<F: Field>() -> CrossTableLookup<F> {
    let keccak_sponge_looking = TableWithColumns::new(
        Table::KeccakSponge,
        keccak_sponge_stark::ctl_looking_keccak_inputs(),
        Some(keccak_sponge_stark::ctl_looking_keccak_filter()),
    );
    let keccak_looked = TableWithColumns::new(
        Table::Keccak,
        keccak_stark::ctl_data_inputs(),
        Some(keccak_stark::ctl_filter_inputs()),
    );
    CrossTableLookup::new(vec![keccak_sponge_looking], keccak_looked)
}

/// `CrossTableLookup` for `KeccakStark` outputs, to connect it with the `KeccakSponge` module.
/// `KeccakStarkSponge` looks into `KeccakStark` to give the outputs of the sponge.
fn ctl_keccak_outputs<F: Field>() -> CrossTableLookup<F> {
    let keccak_sponge_looking = TableWithColumns::new(
        Table::KeccakSponge,
        keccak_sponge_stark::ctl_looking_keccak_outputs(),
        Some(keccak_sponge_stark::ctl_looking_keccak_filter()),
    );
    let keccak_looked = TableWithColumns::new(
        Table::Keccak,
        keccak_stark::ctl_data_outputs(),
        Some(keccak_stark::ctl_filter_outputs()),
    );
    CrossTableLookup::new(vec![keccak_sponge_looking], keccak_looked)
}

/// `CrossTableLookup` for `KeccakSpongeStark` to connect it with the `Cpu` module.
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

/// `CrossTableLookup` for `LogicStark` to connect it with the `Cpu` and `KeccakSponge` modules.
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

/// `CrossTableLookup` for `MemoryStark` to connect it with all the modules which need memory accesses.
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
    let cpu_push_write_ops = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_partial_memory::<F>(),
        Some(cpu_stark::ctl_filter_partial_memory()),
    );
    let cpu_set_context_write = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_memory_old_sp_write_set_context::<F>(),
        Some(cpu_stark::ctl_filter_set_context()),
    );
    let cpu_set_context_read = TableWithColumns::new(
        Table::Cpu,
        cpu_stark::ctl_data_memory_new_sp_read_set_context::<F>(),
        Some(cpu_stark::ctl_filter_set_context()),
    );
    let keccak_sponge_reads = (0..KECCAK_RATE_BYTES).map(|i| {
        TableWithColumns::new(
            Table::KeccakSponge,
            keccak_sponge_stark::ctl_looking_memory(i),
            Some(keccak_sponge_stark::ctl_looking_memory_filter(i)),
        )
    });
    let byte_packing_ops = (0..32).map(|i| {
        TableWithColumns::new(
            Table::BytePacking,
            byte_packing_stark::ctl_looking_memory(i),
            Some(byte_packing_stark::ctl_looking_memory_filter(i)),
        )
    });
    let all_lookers = vec![
        cpu_memory_code_read,
        cpu_push_write_ops,
        cpu_set_context_write,
        cpu_set_context_read,
    ]
    .into_iter()
    .chain(cpu_memory_gp_ops)
    .chain(keccak_sponge_reads)
    .chain(byte_packing_ops)
    .collect();
    let memory_looked = TableWithColumns::new(
        Table::Memory,
        memory_stark::ctl_data(),
        Some(memory_stark::ctl_filter()),
    );
    CrossTableLookup::new(all_lookers, memory_looked)
}
