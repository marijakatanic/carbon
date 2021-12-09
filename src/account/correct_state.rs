use crate::{
    account::{operations::Withdraw, Operation, OperationError},
    commit::Payload,
};

use doomstack::{here, Doom, ResultExt, Top};

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

    pub fn apply(
        &mut self,
        payload: &Payload,
        _dependency: Option<&Operation>,
    ) -> Result<(), Top<OperationError>> {
        if payload.height() <= self.height {
            // `payload` was previously applied with success
            return Ok(());
        }

        // Assuming that `payload.height()` is applicable, we have
        // `payload.height() == self.height + 1`

        let result = match payload.operation() {
            Operation::Withdraw(withdraw) => self.process_withdraw(withdraw),
        };

        if result.is_ok() {
            self.height = payload.height();
        }

        result
    }

    fn process_withdraw(&mut self, withdraw: &Withdraw) -> Result<(), Top<OperationError>> {
        if withdraw.amount() <= self.balance {
            self.balance -= withdraw.amount();
            Ok(())
        } else {
            OperationError::Overdraft.fail().spot(here!())
        }
    }
}
