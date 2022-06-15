// TODO: Dummy code for now.
function storage_read {
    JUMPDEST
    PUSH 1234
    POP

    // An infinite loop:
  mylabel:
    JUMPDEST
    PUSH mylabel
    JUMP
}
