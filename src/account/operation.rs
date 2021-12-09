use crate::{account::Id, crypto::Identify};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::{self, Hash};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Operation {
    Mint {
        amount: u64,
        // TODO: Add proof
    },
    Withdraw {
        amount: u64,
        recipient: Id,
        slot: u64,
    },
    Deposit {},
    Collect {},
    Support {},
    Abandon {},
}

impl Identify for Operation {
    fn identifier(&self) -> Hash {
        hash::hash(self).unwrap()
    }
}
