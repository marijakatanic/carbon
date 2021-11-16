use talk::crypto::primitives::multi::Signature as MultiSignature;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) struct SummarizeConfirm {
    pub signature: MultiSignature,
}
