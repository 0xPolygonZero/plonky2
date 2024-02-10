%macro call_charge_gas(is_call_or_callcode, is_call_or_staticcall)
    %stack (cold_access, address, gas, kexit_info, value) ->
        ($is_call_or_callcode, $is_call_or_staticcall, cold_access, address, gas, kexit_info, value, %%after)
    %jump(call_charge_gas)
%%after:
    //  stack: kexit_info, C_callgas, address, value
%endmacro

// Charge gas for *call opcodes and return the sub-context gas limit.
// Doesn't include memory expansion costs.
global call_charge_gas:
    // Compute C_access
    // stack: is_call_or_callcode, is_call_or_staticcall, cold_access, address, gas, kexit_info, value, retdest
    SWAP2
    // stack: cold_access, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %mul_const(@GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS)
    %add_const(@GAS_WARMACCESS)
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    DUP3
    // stack: is_call_or_callcode, cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %jumpi(xfer_cost)
after_xfer_cost:
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    DUP2
    %jumpi(new_cost)
after_new_cost:
    %stack (Cextra, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest) ->
        (Cextra, address, gas, kexit_info, value, retdest)
    // Compute C_gascap
    // stack: Cextra, address, gas, kexit_info, value, retdest
    DUP4 %leftover_gas
    // stack: leftover_gas, Cextra, address, gas, kexit_info, value, retdest
    DUP2 DUP2 LT
    // stack: leftover_gas<Cextra, leftover_gas, Cextra, address, gas, kexit_info, value, retdest
    DUP5 DUP2 MUL
    // stack: (leftover_gas<Cextra)*gas, leftover_gas<Cextra, leftover_gas, Cextra, address, gas, kexit_info, value, retdest
    SWAP1 %not_bit
    // stack: leftover_gas>=Cextra, (leftover_gas<Cextra)*gas, leftover_gas, Cextra, address, gas, kexit_info, value, retdest
    DUP4 DUP4 SUB
    // stack: leftover_gas - Cextra, leftover_gas>=Cextra, (leftover_gas<Cextra)*gas, leftover_gas, Cextra, address, gas, kexit_info, value, retdest
    %all_but_one_64th
    // stack: L(leftover_gas - Cextra), leftover_gas>=Cextra, (leftover_gas<Cextra)*gas, leftover_gas, Cextra, address, gas, kexit_info, value, retdest
    DUP7 %min MUL ADD
    // stack: Cgascap, leftover_gas, Cextra, address, gas, kexit_info, value, retdest

    // Compute C_call and charge for it.
    %stack (Cgascap, leftover_gas, Cextra) -> (Cextra, Cgascap, Cgascap)
    ADD
    %stack (C_call, Cgascap, address, gas, kexit_info, value) ->
        (C_call, kexit_info, Cgascap, address, gas, value)
    %charge_gas

    // Compute C_callgas
    %stack (kexit_info, Cgascap, address, gas, value) ->
        (Cgascap, address, gas, kexit_info, value)
    DUP5 ISZERO %not_bit
    // stack: value!=0, Cgascap, address, gas, kexit_info, value, retdest
    %mul_const(@GAS_CALLSTIPEND) ADD
    %stack (C_callgas, address, gas, kexit_info, value, retdest) ->
        (retdest, kexit_info, C_callgas, address, value)
    JUMP

global xfer_cost:
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    DUP7
    // stack: value, cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %jumpi(xfer_cost_nonzero)
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %jump(after_xfer_cost)
xfer_cost_nonzero:
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %add_const(@GAS_CALLVALUE)
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %jump(after_xfer_cost)

new_cost:
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    DUP7
    // stack: value, cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %jumpi(new_cost_transfers_value)
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %jump(after_new_cost)
new_cost_transfers_value:
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    DUP4 %is_dead
    %jumpi(new_cost_nonzero)
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %jump(after_new_cost)
new_cost_nonzero:
    // stack: cost, is_call_or_staticcall, is_call_or_callcode, address, gas, kexit_info, value, retdest
    %add_const(@GAS_NEWACCOUNT)
    %jump(after_new_cost)
