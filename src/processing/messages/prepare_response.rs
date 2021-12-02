use crate::account::Id;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;

#[derive(Serialize, Deserialize)]
pub(crate) enum PrepareResponse {
    UnknownIds(Vec<Id>),
    WitnessShard(MultiSignature),
}
