use crate::account::operations::{Abandon, Collect, Deposit, Mint, Support, Withdraw};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Operation {
    Mint(Mint),
    Withdraw(Withdraw),
    Deposit(Deposit),
    Collect(Collect),
    Support(Support),
    Abandon(Abandon),
}
