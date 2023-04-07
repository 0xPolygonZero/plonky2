global precompile_ecrec:
    %stack (address, retdest, address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size) ->
        (kexit_info, args_offset, args_size, ret_offset, ret_size)

    PUSH @ECREC_GAS %charge_gas
    %stack (kexit_info, args_offset, args_size, ret_offset, ret_size) ->
        (args_size, args_offset, args_size, ret_offset, ret_size, kexit_info)

// Pre stack: addr: 3, len, retdest
// Post stack: packed_value
// NOTE: addr: 3 denotes a (context, segment, virtual) tuple
