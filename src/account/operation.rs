use crate::{
    account::{operations::Withdraw, Id},
    crypto::Identify,
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::{self, Hash};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Operation {
    Withdraw(Withdraw),
}

impl Operation {
    pub fn withdraw(beneficiary: Id, slot: u64, amount: u64) -> Self {
        Operation::Withdraw(Withdraw::new(beneficiary, slot, amount))
    }
}

impl Identify for Operation {
    fn identifier(&self) -> Hash {
        hash::hash(self).unwrap()
    }
}
