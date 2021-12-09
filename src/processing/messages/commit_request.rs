use serde::{Deserialize, Serialize};

use crate::{
    commit::{CommitProof, Payload},
    crypto::Certificate,
};

#[derive(Serialize, Deserialize)]
pub(crate) enum CommitRequest {
    Ping,
    Batch(Vec<Payload>),
    WitnessRequest,
    CommitProofs(Vec<CommitProof>),
    Witness(Certificate),
}
