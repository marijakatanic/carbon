use crate::{
    account::{Entry, Operation},
    commit::Payload,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash::Hash;

use zebra::map::Set;

pub(crate) enum State {
    Correct {
        height: u64,
        balance: u64,
        deposits: Deposits,
        motions: BTreeSet<Hash>,
    },
    Corrupted(u64),
}

pub(crate) struct Deposits {
    slot: u64,
    root: Option<Hash>,
}

#[derive(Doom)]
pub(crate) enum StateError {
    #[doom(description("Previously corrupted"))]
    PreviouslyCorrupted,
    #[doom(description("Overdraft"))]
    Overdraft,
}

impl State {
    pub fn new() -> Self {
        State::Correct {
            height: 0,
            balance: 0,
            deposits: Deposits {
                slot: 0,
                root: None,
            },
            motions: BTreeSet::new(),
        }
    }

    pub fn applicable(&mut self, payload: &Payload) -> bool {
        match self {
            State::Correct { height, .. } => payload.height() <= *height + 1,
            State::Corrupted(_) => true,
        }
    }

    // This function should only be called if `payload` is valid and applicable
    pub fn apply(&mut self, payload: &Payload) -> bool {
        let result = (|| match self {
            State::Correct {
                height, balance, ..
            } => {
                if payload.height() <= *height {
                    // `payload` was already successfully applied to `self`
                    return Ok(());
                }

                match payload.operation() {
                    Operation::Withdraw { amount, .. } => {
                        if *balance >= *amount {
                            *balance -= *amount;
                            Ok(())
                        } else {
                            StateError::Overdraft.fail().spot(here!())
                        }
                    }
                    _ => todo!(),
                }
            }
            State::Corrupted(_) => StateError::PreviouslyCorrupted.fail().spot(here!()),
        })();

        todo!()
    }

    pub fn corrupt(&mut self, height: u64) {
        match self {
            State::Correct { .. } => {
                *self = State::Corrupted(height);
            }
            State::Corrupted(_) => {}
        }
    }
}
