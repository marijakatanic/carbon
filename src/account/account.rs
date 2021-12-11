use crate::{
    account::{AccountSettings, CorrectState, Id, Operation, State},
    commit::Payload,
};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Account {
    height: u64,
    state: State,
}

impl Account {
    pub fn new(id: Id, settings: &AccountSettings) -> Self {
        Account {
            height: 0,
            state: State::Correct(CorrectState::new(id, settings)),
        }
    }

    pub fn applicable(&self, height: u64) -> bool {
        height <= self.height + 1
    }

    pub fn apply(
        &mut self,
        payload: &Payload,
        dependency: Option<&Operation>,
        settings: &AccountSettings,
    ) -> bool {
        if payload.height() <= self.height {
            return payload.height() < self.height || self.state.is_correct();
        }

        let result = match &mut self.state {
            State::Correct(state) => state.apply(payload.operation(), dependency, settings),
            State::Corrupted(_) => unreachable!(),
        };

        self.height += 1;

        match result {
            Ok(()) => true,
            Err(_) => {
                self.state.corrupt();
                false
            }
        }
    }
}
