use crate::account::{AccountSettings, CorrectState, Id, State};

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
}
