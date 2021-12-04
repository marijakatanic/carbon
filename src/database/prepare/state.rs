use crate::{database::prepare::PrepareHandle, prepare::Equivocation};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone)]
pub(crate) enum State {
    Consistent {
        height: u64,
        commitment: Hash,
        handle: PrepareHandle,
    },
    Equivocated(Equivocation),
}
