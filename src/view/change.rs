use serde::{Deserialize, Serialize};

use talk::crypto::primitives::sign::PublicKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Change {
    Join(PublicKey),
    Leave(PublicKey),
}
