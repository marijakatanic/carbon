use std::collections::BTreeSet;

use talk::crypto::primitives::hash::Hash;

pub(crate) enum State {
    Correct {
        height: u64,
        balance: u64,
        deposits: Deposits,
        motions: BTreeSet<Hash>,
    },
    Corrupted,
}

pub(crate) struct Deposits {
    slot: u64,
    root: Hash,
}
