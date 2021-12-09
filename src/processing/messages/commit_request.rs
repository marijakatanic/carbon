use serde::{Deserialize, Serialize};

use zebra::vector::Vector;

use crate::{
    commit::{CommitProof, Payload},
    crypto::Certificate,
};

#[derive(Serialize, Deserialize)]
pub(crate) enum CommitRequest {
    Ping,
    Batch(Vector<Payload>),
    WitnessRequest,
    CommitProofs(Vec<CommitProof>),
    Witness(Certificate),
}
