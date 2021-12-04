use crate::{account::Id, prepare::BatchCommitShard};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;

#[derive(Serialize, Deserialize)]
pub(crate) enum PrepareResponse {
    Pong,
    UnknownIds(Vec<Id>),
    WitnessShard(MultiSignature),
    CommitShard(BatchCommitShard),
}
