use crate::{account::Id, commit::BatchCompletionShard};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;

#[derive(Serialize, Deserialize)]
pub(crate) enum CommitResponse {
    Pong,
    MissingProofs(Vec<Id>),
    WitnessShard(MultiSignature),
    CompletionShard(BatchCompletionShard),
}
