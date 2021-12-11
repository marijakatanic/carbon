use talk::crypto::primitives::hash::Hash;

pub(crate) struct PayloadHandle {
    pub batch: Hash,
    pub index: usize,
}
