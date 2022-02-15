/// The previous state root, before these transactions were executed.
const PI_OLD_STATE_ROOT: usize = 0;

/// The updated state root, after these transactions were executed.
const PI_NEW_STATE_ROOT: usize = PI_OLD_STATE_ROOT + 1;

pub(crate) const NUM_PUBLIC_INPUTS: usize = PI_NEW_STATE_ROOT + 1;
