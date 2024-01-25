log2_floor_helper:
    // stack: val, counter, retdest
    DUP1
    // stack: val, val, counter, retdest
    ISZERO
    %jumpi(end)
    // stack: val, counter, retdest
    %div2
    // stack: val/2, counter, retdest
    SWAP1
    %increment
    SWAP1
    // stack: val/2, counter + 1, retdest
    %jump(log2_floor_helper)
end:
    // stack: val, counter, retdest
    POP
    // stack: counter, retdest
    SWAP1
    // stack: retdest, counter
    JUMP

global log2_floor:
    // stack: val, retdest
    %div2
    // stack: val/2, retdest
    PUSH 0
    // stack: 0, val/2, retdest
    SWAP1
    // stack: val/2, 0, retdest
    %jump(log2_floor_helper)

%macro log2_floor
    %stack (val) -> (val, %%after)
    %jump(log2_floor)
%%after:
%endmacro
