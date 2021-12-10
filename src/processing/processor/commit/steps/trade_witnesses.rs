use crate::{
    crypto::Certificate,
    processing::{
        messages::{CommitRequest, CommitResponse},
        processor::commit::errors::ServeCommitError,
    },
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::primitives::multi::Signature as MultiSignature, net::Session};

pub(in crate::processing::processor::commit) async fn trade_witnesses(
    session: &mut Session,
    shard: MultiSignature,
) -> Result<Certificate, Top<ServeCommitError>> {
    // Send witness `shard`

    session
        .send(&CommitResponse::WitnessShard(shard))
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    // Receive witness certificate (which aggregates a plurality of witness
    // shards produced by other members of the replica's view)

    let request = session
        .receive::<CommitRequest>()
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    let witness = match request {
        CommitRequest::Witness(witness) => witness,
        _ => {
            return ServeCommitError::UnexpectedRequest.fail().spot(here!());
        }
    };

    // The verification of `witness` is delegated to the caller `witnessed_batch(..)`,
    // which validates the `WitnessedBatch` it builds from `witness`

    Ok(witness)
}
