use crate::{crypto::Certificate, prepare::Batch, signup::IdAssignment};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum PrepareRequest {
    Batch(Batch),
    WitnessShardRequest,
    IdAssignments(Vec<IdAssignment>),
    Witness(Certificate),
}
