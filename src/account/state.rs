use crate::account::CorrectState;

pub(crate) enum State {
    Correct(CorrectState),
    Corrupted,
}

impl State {}
