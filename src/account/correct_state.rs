use std::collections::HashSet;

use talk::crypto::primitives::hash::Hash;

pub(crate) struct CorrectState {
    height: u64,
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
            height: 0,
            balance: 0,
            deposits: Deposits {
                slot: 0,
                root: None,
            },
            motions: HashSet::new(),
        }
    }
}
