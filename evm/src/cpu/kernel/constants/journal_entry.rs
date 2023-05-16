#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum JournalEntry {
    AccountLoaded = 0,
    AccountDestroyed = 1,
    AccountTouched = 2,
    BalanceTransfer = 3,
    NonceChange = 4,
    StorageChange = 5,
    StorageLoaded = 6,
    CodeChange = 7,
    Refund = 8,
    AccountCreated = 9,
}

impl JournalEntry {
    pub(crate) const COUNT: usize = 10;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [
            Self::AccountLoaded,
            Self::AccountDestroyed,
            Self::AccountTouched,
            Self::BalanceTransfer,
            Self::NonceChange,
            Self::StorageChange,
            Self::StorageLoaded,
            Self::CodeChange,
            Self::Refund,
            Self::AccountCreated,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            Self::AccountLoaded => "JOURNAL_ENTRY_ACCOUNT_LOADED",
            Self::AccountDestroyed => "JOURNAL_ENTRY_ACCOUNT_DESTROYED",
            Self::AccountTouched => "JOURNAL_ENTRY_ACCOUNT_TOUCHED",
            Self::BalanceTransfer => "JOURNAL_ENTRY_BALANCE_TRANSFER",
            Self::NonceChange => "JOURNAL_ENTRY_NONCE_CHANGE",
            Self::StorageChange => "JOURNAL_ENTRY_STORAGE_CHANGE",
            Self::StorageLoaded => "JOURNAL_ENTRY_STORAGE_LOADED",
            Self::CodeChange => "JOURNAL_ENTRY_CODE_CHANGE",
            Self::Refund => "JOURNAL_ENTRY_REFUND",
            Self::AccountCreated => "JOURNAL_ENTRY_ACCOUNT_CREATED",
        }
    }
}
