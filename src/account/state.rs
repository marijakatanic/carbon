use crate::{
    account::{CorrectState, CorruptedState, StateSummary},
    crypto::Identify,
};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) enum State {
    Correct(CorrectState),
    Corrupted(CorruptedState),
}

impl State {
    pub fn is_correct(&self) -> bool {
        match self {
            State::Correct(_) => true,
            _ => false,
        }
    }

    pub fn is_corrupted(&self) -> bool {
        match self {
            State::Corrupted(_) => true,
            _ => false,
        }
    }

    pub fn summarize(&self) -> StateSummary {
        match self {
            State::Correct(state) => StateSummary::Correct(state.identifier()),
            State::Corrupted(_) => StateSummary::Corrupted,
        }
    }

    pub fn corrupt(&mut self) {
        if let State::Correct(state) = self {
            *self = State::Corrupted(state.corrupted());
        }
    }
}
