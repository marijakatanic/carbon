use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum StateSummary {
    Correct(Hash),
    Corrupted,
}
