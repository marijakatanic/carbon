use crate::account::operations::{Abandon, Claim, Collect, Deposit, Mint, Support, Withdraw};

pub(crate) enum Operation {
    Claim(Claim),
    Mint(Mint),
    Withdraw(Withdraw),
    Deposit(Deposit),
    Support(Support),
    Abandon(Abandon),
    Collect(Collect),
}
