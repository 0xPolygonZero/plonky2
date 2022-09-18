%macro load_from_block
    // stack: block, r
    ADD
    // stack: offset = block + r
%endmacro

%macro init_buffer

%endmacro

%macro store_input
    // stack: ADDR
%endmacro

%macro store_padding
%endmacro

%macro store_size
    // stack: length
    %shl_const(3)
    // stack: length
%endmacro
