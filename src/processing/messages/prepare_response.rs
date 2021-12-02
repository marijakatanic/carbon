use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;

#[derive(Serialize, Deserialize)]
pub(crate) enum PrepareResponse {
    WitnessShard(MultiSignature),
}
