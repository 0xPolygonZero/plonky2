// Load a byte from the given segment of the current context's memory space.
// Note that main memory values are one byte each, but in general memory values
// can be 256 bits. This macro deals with a single address (unlike MSTORE), so
// if it is used with main memory, it will load a single byte.
%macro mload_current(segment)
    // stack: offset
    PUSH $segment
    // stack: segment, offset
    CURRENT_CONTEXT
    // stack: context, segment, offset
    MLOAD_GENERAL
    // stack: value
%endmacro

// Store a byte to the given segment of the current context's memory space.
// Note that main memory values are one byte each, but in general memory values
// can be 256 bits. This macro deals with a single address (unlike MSTORE), so
// if it is used with main memory, it will store a single byte.
%macro mstore_current(segment)
    // stack: offset, value
    PUSH $segment
    // stack: segment, offset, value
    CURRENT_CONTEXT
    // stack: context, segment, offset, value
    MSTORE_GENERAL
    // stack: (empty)
%endmacro
