// struct StorageLoaded { address, slot }

%macro journal_add_storage_loaded
    %journal_add_2(@JOURNAL_ENTRY_STORAGE_LOADED)
%endmacro

global revert_storage_loaded:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_2
    // stack: address, slot, retdest
    %jump(remove_accessed_storage_keys)
