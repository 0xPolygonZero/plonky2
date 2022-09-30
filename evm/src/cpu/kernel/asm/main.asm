global main:
    // If the prover has no more txns for us to process, halt.
    PROVER_INPUT(end_of_txns)
    %jumpi(halt)

    // Call route_txn, returning to main to continue the loop.
    PUSH main
    %jump(route_txn)
