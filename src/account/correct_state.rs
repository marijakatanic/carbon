use std::collections::HashSet;

use talk::crypto::primitives::hash::Hash;

pub(crate) struct CorrectState {
    balance: u64,
    deposits: Deposits,
    motions: HashSet<Hash>,
}

pub(crate) struct Deposits {
    slot: u64,
    root: Option<Hash>,
}

impl CorrectState {
    pub fn new() -> Self {
        CorrectState {
            balance: 0,
            deposits: Deposits {
                slot: 0,
                root: None,
            },
            motions: HashSet::new(),
        }
    }
}
