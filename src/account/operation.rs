use crate::{
    account::operations::{Abandon, Collect, Deposit, Mint, Support, Withdraw},
    crypto::Identify,
};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::{self, Hash};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Operation {
    Mint(Mint),
    Withdraw(Withdraw),
    Deposit(Deposit),
    Collect(Collect),
    Support(Support),
    Abandon(Abandon),
}

impl Identify for Operation {
    fn identifier(&self) -> Hash {
        hash::hash(self).unwrap()
    }
}
