use crate::account::{
    operations::{Abandon, Deposit, Support, Withdraw},
    Operation, OperationError,
};

use doomstack::{here, Doom, ResultExt, Top};

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

    pub fn apply(
        &mut self,
        operation: &Operation,
        dependency: Option<&Operation>,
    ) -> Result<(), Top<OperationError>> {
        match operation {
            Operation::Withdraw(withdraw) => self.apply_withdraw(withdraw),
            Operation::Deposit(deposit) => self.apply_deposit(deposit, dependency.unwrap()),
            Operation::Support(support) => self.apply_support(support),
            Operation::Abandon(abandon) => self.apply_abandon(abandon),
        }
    }

    fn apply_withdraw(&mut self, withdraw: &Withdraw) -> Result<(), Top<OperationError>> {
        todo!()
    }

    fn apply_deposit(
        &mut self,
        deposit: &Deposit,
        dependency: &Operation,
    ) -> Result<(), Top<OperationError>> {
        todo!()
    }

    fn apply_support(&mut self, support: &Support) -> Result<(), Top<OperationError>> {
        todo!()
    }

    fn apply_abandon(&mut self, abandon: &Abandon) -> Result<(), Top<OperationError>> {
        todo!()
    }
}
