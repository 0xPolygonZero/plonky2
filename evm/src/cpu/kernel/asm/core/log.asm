// TODO: Implement receipts

global sys_log0:
    %check_static
    // stack: kexit_info, offset, size
    DUP3 ISZERO %jumpi(log0_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log0_after_mem_gas:
    // stack: kexit_info, offset, size
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG)
    // stack: gas, kexit_info, offset, size
    %charge_gas
    %stack (kexit_info, offset, size) -> (kexit_info)
    EXIT_KERNEL

global sys_log1:
    %check_static
    // stack: kexit_info, offset, size, topic
    DUP3 ISZERO %jumpi(log1_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log1_after_mem_gas:
    // stack: kexit_info, offset, size, topic
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic
    %charge_gas
    %stack (kexit_info, offset, size, topic) -> (kexit_info)
    EXIT_KERNEL

global sys_log2:
    %check_static
    // stack: kexit_info, offset, size, topic1, topic2
    DUP3 ISZERO %jumpi(log2_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic1, topic2
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log2_after_mem_gas:
    // stack: kexit_info, offset, size, topic1, topic2
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic1, topic2
    %charge_gas
    %stack (kexit_info, offset, size, topic1, topic2) -> (kexit_info)
    EXIT_KERNEL

global sys_log3:
    %check_static
    // stack: kexit_info, offset, size, topic1, topic2, topic3
    DUP3 ISZERO %jumpi(log3_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic1, topic2, topic3
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log3_after_mem_gas:
    // stack: kexit_info, offset, size, topic1, topic2, topic3
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic1, topic2, topic3
    %charge_gas
    %stack (kexit_info, offset, size, topic1, topic2, topic3) -> (kexit_info)
    EXIT_KERNEL

global sys_log4:
    %check_static
    // stack: kexit_info, offset, size, topic1, topic2, topic3, topic4
    DUP3 ISZERO %jumpi(log4_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic1, topic2, topic3, topic4
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log4_after_mem_gas:
    // stack: kexit_info, offset, size, topic1, topic2, topic3, topic4
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic1, topic2, topic3, topic4
    %charge_gas
    %stack (kexit_info, offset, size, topic1, topic2, topic3, topic4) -> (kexit_info)
    EXIT_KERNEL
