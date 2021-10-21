use serde::{Deserialize, Serialize};

use std::hash::Hash;

use talk::crypto::primitives::sign::PublicKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Change {
    Join(PublicKey),
    Leave(PublicKey),
}
