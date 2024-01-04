use crate::memory::segments::Segment;

/// These are normalized transaction fields, i.e. not specific to any transaction type.
///
/// Each value is directly scaled by the corresponding `Segment::TxnFields` value for faster
/// memory access in the kernel.
#[allow(dead_code)]
#[allow(clippy::enum_clike_unportable_variant)]
#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum NormalizedTxnField {
    /// Whether a chain ID was present in the txn data. Type 0 transaction with v=27 or v=28 have
    /// no chain ID. This affects what fields get signed.
    ChainIdPresent = Segment::TxnFields as usize,
    ChainId,
    Nonce,
    MaxPriorityFeePerGas,
    MaxFeePerGas,
    GasLimit,
    IntrinsicGas,
    To,
    Value,
    /// The length of the data field. The data itself is stored in another segment.
    DataLen,
    YParity,
    R,
    S,
    Origin,

    /// The actual computed gas price for this transaction in the block.
    /// This is not technically a transaction field, as it depends on the block's base fee.
    ComputedFeePerGas,
    ComputedPriorityFeePerGas,
}

impl NormalizedTxnField {
    pub(crate) const COUNT: usize = 16;

    /// Unscales this virtual offset by their respective `Segment` value.
    pub(crate) const fn unscale(&self) -> usize {
        *self as usize - Segment::TxnFields as usize
    }

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        [
            Self::ChainIdPresent,
            Self::ChainId,
            Self::Nonce,
            Self::MaxPriorityFeePerGas,
            Self::MaxFeePerGas,
            Self::GasLimit,
            Self::IntrinsicGas,
            Self::To,
            Self::Value,
            Self::DataLen,
            Self::YParity,
            Self::R,
            Self::S,
            Self::Origin,
            Self::ComputedFeePerGas,
            Self::ComputedPriorityFeePerGas,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) const fn var_name(&self) -> &'static str {
        match self {
            NormalizedTxnField::ChainIdPresent => "TXN_FIELD_CHAIN_ID_PRESENT",
            NormalizedTxnField::ChainId => "TXN_FIELD_CHAIN_ID",
            NormalizedTxnField::Nonce => "TXN_FIELD_NONCE",
            NormalizedTxnField::MaxPriorityFeePerGas => "TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS",
            NormalizedTxnField::MaxFeePerGas => "TXN_FIELD_MAX_FEE_PER_GAS",
            NormalizedTxnField::GasLimit => "TXN_FIELD_GAS_LIMIT",
            NormalizedTxnField::IntrinsicGas => "TXN_FIELD_INTRINSIC_GAS",
            NormalizedTxnField::To => "TXN_FIELD_TO",
            NormalizedTxnField::Value => "TXN_FIELD_VALUE",
            NormalizedTxnField::DataLen => "TXN_FIELD_DATA_LEN",
            NormalizedTxnField::YParity => "TXN_FIELD_Y_PARITY",
            NormalizedTxnField::R => "TXN_FIELD_R",
            NormalizedTxnField::S => "TXN_FIELD_S",
            NormalizedTxnField::Origin => "TXN_FIELD_ORIGIN",
            NormalizedTxnField::ComputedFeePerGas => "TXN_FIELD_COMPUTED_FEE_PER_GAS",
            NormalizedTxnField::ComputedPriorityFeePerGas => {
                "TXN_FIELD_COMPUTED_PRIORITY_FEE_PER_GAS"
            }
        }
    }
}
