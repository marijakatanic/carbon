use crate::account::operations::{Abandon, Collect, Deposit, Mint, Support, Withdraw};

pub(crate) enum Operation {
    Mint(Mint),
    Withdraw(Withdraw),
    Deposit(Deposit),
    Support(Support),
    Abandon(Abandon),
    Collect(Collect),
}
