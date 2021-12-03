use crate::prepare::Extract;

use talk::crypto::primitives::hash::Hash;

pub(crate) enum PrepareHandle {
    Batched { batch: Hash, index: usize },
    Standalone(Extract),
}
