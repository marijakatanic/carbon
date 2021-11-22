use talk::crypto::primitives::hash::Hash;
use talk::crypto::KeyCard;

pub(crate) enum State {
    Correct {
        keycard: KeyCard,
        height: u64,
        balance: u64,
        deposit: Deposit,
        motions: Vec<Hash>,
    },
    Corrupted,
}

pub(crate) struct Deposit {
    slot: u64,
    root: Hash,
}
