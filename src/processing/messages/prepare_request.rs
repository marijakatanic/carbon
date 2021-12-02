use crate::{crypto::Certificate, prepare::Batch};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum PrepareRequest {
    Batch(Batch),
    WitnessShardRequest,
    Witness(Certificate),
}
