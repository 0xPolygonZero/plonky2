/// These are normalized transaction fields, i.e. not specific to any transaction type.
#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum NormalizedTxnField {
    /// Whether a chain ID was present in the txn data. Type 0 transaction with v=27 or v=28 have
    /// no chain ID. This affects what fields get signed.
    ChainIdPresent = 0,
    ChainId = 1,
    Nonce = 2,
    MaxPriorityFeePerGas = 3,
    MaxFeePerGas = 4,
    GasLimit = 5,
    To = 6,
    Value = 7,
    /// The length of the data field. The data itself is stored in another segment.
    DataLen = 8,
    YParity = 9,
    R = 10,
    S = 11,
}

impl NormalizedTxnField {
    pub(crate) const COUNT: usize = 12;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [
            Self::ChainIdPresent,
            Self::ChainId,
            Self::Nonce,
            Self::MaxPriorityFeePerGas,
            Self::MaxFeePerGas,
            Self::GasLimit,
            Self::To,
            Self::Value,
            Self::DataLen,
            Self::YParity,
            Self::R,
            Self::S,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            NormalizedTxnField::ChainIdPresent => "TXN_FIELD_CHAIN_ID_PRESENT",
            NormalizedTxnField::ChainId => "TXN_FIELD_CHAIN_ID",
            NormalizedTxnField::Nonce => "TXN_FIELD_NONCE",
            NormalizedTxnField::MaxPriorityFeePerGas => "TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS",
            NormalizedTxnField::MaxFeePerGas => "TXN_FIELD_MAX_FEE_PER_GAS",
            NormalizedTxnField::GasLimit => "TXN_FIELD_GAS_LIMIT",
            NormalizedTxnField::To => "TXN_FIELD_TO",
            NormalizedTxnField::Value => "TXN_FIELD_VALUE",
            NormalizedTxnField::DataLen => "TXN_FIELD_DATA_LEN",
            NormalizedTxnField::YParity => "TXN_FIELD_Y_PARITY",
            NormalizedTxnField::R => "TXN_FIELD_R",
            NormalizedTxnField::S => "TXN_FIELD_S",
        }
    }
}
