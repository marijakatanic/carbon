use crate::account::CorrectState;

pub(crate) enum State {
    Correct(CorrectState),
    Corrupted(u64),
}

impl State {}
