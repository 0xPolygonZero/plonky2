// Increment by 1 the rlp encoded index and increment
// its number of nibbles when required. Shouldn't be
// called with rlp_index > 0x82 ff ff
global increment_bounded_rlp:
    // stack: rlp_index, num_nibbles, retdest
    DUP1
    %eq_const(0x80)
    %jumpi(case_0x80)
    DUP1
    %eq_const(0x7f)
    %jumpi(case_0x7f)
    DUP1
    %eq_const(0x81ff)
    %jumpi(case_0x81ff)
    // If rlp_index != 0x80 and rlp_index != 0x7f and rlp_index != 0x81ff
    // we only need to add one and keep the number of nibbles
    %increment
    %stack (rlp_index, num_nibbles, retdest) -> (retdest, rlp_index, num_nibbles)
    JUMP

case_0x80:
    %stack (rlp_index, num_nibbles, retdest) -> (retdest, 0x01, 2)
    JUMP
case_0x7f:
    %stack (rlp_index, num_nibbles, retdest) -> (retdest, 0x8180, 4)
    JUMP

case_0x81ff:
    %stack (rlp_index, num_nibbles, retdest) -> (retdest, 0x820100, 6)
    JUMP
    
    

%macro increment_bounded_rlp
    %stack (rlp_index, num_nibbles) -> (rlp_index, num_nibbles, %%after)
    %jump(increment_bounded_rlp)
%%after:
%endmacro
