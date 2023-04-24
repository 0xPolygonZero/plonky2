/// def bn254_pairing(pairs: List((Curve, TwistedCurve))) -> Bool:
///     
///     for P, Q in pairs:
///         if not (P.is_valid and Q.is_valid):
///             return @U256_MAX
///     
///     out = 1
///     for P, Q in pairs:
///         out *= miller_loop(P, Q)
///
///     result = bn254_final_exponent(out)
///     return result == @GENERATOR_PAIRING

global bn254_pairing:
    // stack: k, inp, out, retdest 
    DUP1

bn254_input_check:
    // stack:       j    , k, inp 
    DUP1
    ISZERO
    // stack: end?, j    , k, inp
    %jumpi(bn254_pairing_start)
    // stack:       j    , k, inp
    %sub_const(1)
    // stack:       j=j-1, k, inp

    %stack (j, k, inp) -> (j, inp, j, k, inp)
    // stack:        j, inp, j, k, inp
    %mul_const(6)
    ADD
    // stack:  inp_j=inp+6j, j, k, inp
    DUP1
    // stack:  inp_j, inp_j, j, k, inp
    %load_fp254_2
    // stack:    P_j, inp_j, j, k, inp
    %bn_check
    // stack: valid?, inp_j, j, k, inp
    ISZERO
    %jumpi(bn_pairing_invalid_input)
    // stack:         inp_j, j, k, inp
    DUP1
    // stack: inp_j , inp_j, j, k, inp
    %add_const(2)
    // stack: inp_j', inp_j, j, k, inp
    %load_fp254_4
    // stack:    Q_j, inp_j, j, k, inp
    %bn_check_twisted
    // stack: valid?, inp_j, j, k, inp
    ISZERO
    %jumpi(bn_pairing_invalid_input)
    // stack:         inp_j, j, k, inp
    POP
    %jump(bn254_input_check)

bn_pairing_invalid_input:
    // stack:  inp_j, j, k, inp, out, retdest
    %stack (inp_j, j, k, inp, out, retdest) -> (retdest, @U256_MAX)
    JUMP

bn254_pairing_start:
    // stack:      0, k, inp, out,                   retdest
    %stack (j, k, inp, out) -> (out, 1, k, inp, out, bn254_pairing_output_validation, out)
    // stack: out, 1, k, inp, out, final_label, out, retdest
    %mstore_kernel_bn254_pairing
    // stack:         k, inp, out, final_label, out, retdest

bn254_pairing_loop:
    // stack:       k, inp, out, final_label
    DUP1
    ISZERO
    // stack: end?, k, inp, out, final_label
    %jumpi(bn254_final_exponent)
    // stack:       k, inp, out, final_label
    %sub_const(1)
    // stack:   k=k-1, inp, out, final_label

    %stack (k, inp, out) -> (k, inp, 0, mul_fp254_12, 0, out, out, bn254_pairing_loop, k, inp, out)
    // stack: k, inp, 0, mul_fp254_12, 0, out, out, bn254_pairing_loop, k, inp, out, final_label
    %mul_const(6)
    ADD
    // stack:  inp_k, 0, mul_fp254_12, 0, out, out, bn254_pairing_loop, k, inp, out, final_label
    %jump(bn254_miller)


bn254_pairing_output_validation:
    // stack:                 out, retdest
    %push_desired_output
    // stack:    g0, g11..g1, out, retdest
    SWAP12
    // stack:        out, g11..g0, retdest
    PUSH 1
    // stack: check, out, g11..g0, retdest 
    %check_output_term(11)
    // stack: check, out, g10..g0, retdest
    %check_output_term(10)
    // stack: check, out,  g9..g0, retdest
    %check_output_term(9)
    // stack: check, out,  g8..g0, retdest

    %check_output_term(8)
    
    // stack: check, out,  g7..g0, retdest
    %check_output_term(7)
    // stack: check, out,  g6..g0, retdest
    %check_output_term(6)
    
    // stack: check, out,  g5..g0, retdest
    %check_output_term(5)
    // stack: check, out,  g4..g0, retdest
    %check_output_term(4)
    // stack: check, out,  g3..g0, retdest
    %check_output_term(3)
    // stack: check, out,  g2..g0, retdest
    %check_output_term(2)
    // stack: check, out,  g1, g0, retdest
    %check_output_term(1)
    // stack: check, out,      g0, retdest
    %check_output_term(0)
    // stack: check, out,        , retdest
    %stack (check, out, retdest) -> (retdest, check)
    JUMP

%macro check_output_term(j)
    // stack:       check, out, gj
    SWAP2
    // stack:       gj, out, check
    DUP2
    %add_const($j)
    // stack: outj, gj, out, check
    %mload_kernel_bn254_pairing
    // stack:   fj, gj, out, check
    EQ
    // stack:   checkj, out, check
    %stack (checkj, out, check) -> (check, checkj, out)
    // stack:   check, checkj, out
    MUL
    // stack:           check, out
%endmacro

%macro push_desired_output
    PUSH 07708764853296235550302896633598331924671113766219240748172066028946006022854  // g1
    PUSH 17700926755167371005308910210965003607045179123434251133647055306492170438120  // g2
    PUSH 00154397549418641559307524478611787574224314011122269053905755152919215659778  // g3
    PUSH 01984170487336525780293932330785856524432038724373274488958019302386252559231  // g4
    PUSH 03314362000193010715052769662421751145025288853014347901929084743686925091033  // g5
    PUSH 05969572836535217971378806448005698172042029600478282326636924294386246370693  // g6
    PUSH 18564243080196493066086408717287862863335702133957524699743268830525148172506  // g7
    PUSH 17269266067816704782247017427200956927940055030199138534350116254357612253048  // g8
    PUSH 09740411817590043771488498441210821606869449023601574073310485764683435152587  // g9
    PUSH 12727712035316870814661734054996728204626079181372322293888505805399715437139  // g10
    PUSH 20210469749439596480915120057935665765860695731536556057113952828024130849369  // g11
    PUSH 05408068458366290097693809645929734991458199404659878659553047611146680628954  // g0
%endmacro
