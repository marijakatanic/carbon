use crate::account::Id;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Withdraw {
    beneficiary: Id,
    slot: u64,
    amount: u64,
}

impl Withdraw {
    pub fn new(beneficiary: Id, slot: u64, amount: u64) -> Self {
        Withdraw {
            beneficiary,
            slot,
            amount,
        }
    }

    pub fn beneficiary(&self) -> Id {
        self.beneficiary
    }

    pub fn slot(&self) -> u64 {
        self.slot
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}
