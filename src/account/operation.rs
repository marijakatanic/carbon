use crate::{
    account::{
        operations::{Deposit, Withdraw},
        Entry, Id,
    },
    crypto::Identify,
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::{self, Hash};

use zebra::map::Set;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Operation {
    Withdraw(Withdraw),
    Deposit(Deposit),
}

impl Operation {
    pub fn withdraw(beneficiary: Id, slot: u64, amount: u64) -> Self {
        Operation::Withdraw(Withdraw::new(beneficiary, slot, amount))
    }

    pub fn deposit(withdraw: Entry, deposits: Option<&Set<Entry>>, collect: bool) -> Self {
        Operation::Deposit(Deposit::new(withdraw, deposits, collect))
    }
}

impl Identify for Operation {
    fn identifier(&self) -> Hash {
        hash::hash(self).unwrap()
    }
}
