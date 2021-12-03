use crate::{crypto::Certificate, prepare::SignedBatch, signup::IdAssignment};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum PrepareRequest {
    Batch(SignedBatch),
    WitnessShardRequest,
    IdAssignments(Vec<IdAssignment>),
    Witness(Certificate),
}
