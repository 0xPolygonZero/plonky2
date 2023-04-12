// Labels for unimplemented syscalls to make the kernel assemble.
// Each label should be removed from this file once it is implemented.

global sys_blockhash:
    PANIC
global sys_prevrandao:
    // TODO: What semantics will this have for Edge?
    PANIC
global sys_log0:
    PANIC
global sys_log1:
    PANIC
global sys_log2:
    PANIC
global sys_log3:
    PANIC
global sys_log4:
    PANIC
