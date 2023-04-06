global precompile_id:
    %stack (address, retdest, address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size) ->
        //(args_offset, args_size, ret_offset, ret_size, kexit_info)
        (args_size, kexit_info, args_offset, args_size, ret_offset, ret_size)
