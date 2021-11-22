use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) struct SummarizationResponse {
    pub signature: MultiSignature,
}
