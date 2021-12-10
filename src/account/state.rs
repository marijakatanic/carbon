use crate::account::{CorrectState, CorruptedState};

pub(crate) enum State {
    Correct(CorrectState),
    Corrupted(CorruptedState),
}
