// struct AccountCreated { account_type, address }
// account_type is 0 for an EOA, 1 for a contract.

%macro journal_add_account_created
    %journal_add_2(@JOURNAL_ENTRY_ACCOUNT_CREATED)
%endmacro

global revert_account_created:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_2
    // stack: account_type, address, retdest
    %jumpi(decrement_created_contracts_len)

revert_account_finish:
    %delete_account
    JUMP

decrement_created_contracts_len:
    %mload_global_metadata(@GLOBAL_METADATA_CREATED_CONTRACTS_LEN)
    %decrement
    %mstore_global_metadata(@GLOBAL_METADATA_CREATED_CONTRACTS_LEN)
    %jump(revert_account_finish)
