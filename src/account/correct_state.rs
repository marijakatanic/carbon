use crate::account::{
    operations::{Abandon, Deposit, Support, Withdraw},
    AccountSettings, CorruptedState, Id, Operation, OperationError,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashSet;

use talk::crypto::primitives::hash::Hash;

use zebra::map::Set;

pub(crate) struct CorrectState {
    id: Id,
    balance: u64,
    deposits: Deposits,
    motions: HashSet<Hash>,
}

pub(crate) struct Deposits {
    slot: u64,
    root: Option<Hash>,
}

impl CorrectState {
    pub fn new(id: Id) -> Self {
        CorrectState {
            id,
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
        settings: &AccountSettings,
    ) -> Result<(), Top<OperationError>> {
        match operation {
            Operation::Withdraw(withdraw) => self.apply_withdraw(withdraw),
            Operation::Deposit(deposit) => self.apply_deposit(deposit, dependency.unwrap()),
            Operation::Support(support) => self.apply_support(support, settings),
            Operation::Abandon(abandon) => self.apply_abandon(abandon),
        }
    }

    fn apply_withdraw(&mut self, withdraw: &Withdraw) -> Result<(), Top<OperationError>> {
        if self.balance < withdraw.amount() {
            return OperationError::Overdraft.fail().spot(here!());
        }

        self.balance -= withdraw.amount();

        Ok(())
    }

    fn apply_deposit(
        &mut self,
        deposit: &Deposit,
        dependency: &Operation,
    ) -> Result<(), Top<OperationError>> {
        let withdraw = match dependency {
            Operation::Withdraw(withdraw) => withdraw,
            _ => {
                return OperationError::UnexpectedDependency.fail().spot(here!());
            }
        };

        if withdraw.beneficiary() != self.id || withdraw.slot() != self.deposits.slot {
            return OperationError::IllegitimateDeposit.fail().spot(here!());
        }

        let deposits = match (self.deposits.root, deposit.exclusion()) {
            (Some(root), Some(exclusion)) => {
                let mut deposits = Set::root_stub(root);

                deposits
                    .import(exclusion.clone())
                    .pot(OperationError::ExclusionInvalid, here!())?;

                if deposits
                    .contains(&deposit.withdraw())
                    .pot(OperationError::ExclusionInvalid, here!())?
                {
                    return OperationError::DoubleDeposit.fail().spot(here!());
                }

                Some(deposits)
            }
            (None, None) => None,
            _ => {
                return OperationError::ExclusionInvalid.fail().spot(here!());
            }
        };

        self.balance += withdraw.amount();

        if deposit.collect() {
            self.deposits.slot += 1;
            self.deposits.root = None;
        } else {
            let mut deposits = deposits.unwrap_or(Set::new());
            deposits.insert(deposit.withdraw()).unwrap();
            self.deposits.root = Some(deposits.commit());
        }

        Ok(())
    }

    fn apply_support(
        &mut self,
        support: &Support,
        settings: &AccountSettings,
    ) -> Result<(), Top<OperationError>> {
        if self.motions.len() >= settings.supports_capacity {
            return OperationError::MotionsOverflow.fail().spot(here!());
        }

        if !self.motions.insert(support.motion()) {
            return OperationError::DoubleSupport.fail().spot(here!());
        }

        Ok(())
    }

    fn apply_abandon(&mut self, abandon: &Abandon) -> Result<(), Top<OperationError>> {
        if !self.motions.remove(&abandon.motion()) {
            return OperationError::UnexpectedAbandon.fail().spot(here!());
        }

        Ok(())
    }

    pub fn corrupt(self) -> CorruptedState {
        CorruptedState::new(self.id)
    }
}
