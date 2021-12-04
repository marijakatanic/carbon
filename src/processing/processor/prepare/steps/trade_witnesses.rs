use crate::{
    crypto::Certificate,
    processing::{
        messages::{PrepareRequest, PrepareResponse},
        processor::prepare::errors::ServePrepareError,
    },
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::primitives::multi::Signature as MultiSignature, net::Session};

pub(in crate::processing::processor::prepare) async fn trade_witnesses(
    session: &mut Session,
    shard: MultiSignature,
) -> Result<Certificate, Top<ServePrepareError>> {
    // Send witness `shard`

    session
        .send(&PrepareResponse::WitnessShard(shard))
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    // Receive witness certificate (which aggregates a plurality of witness
    // shards produced by other members of the replica's view)

    let request = session
        .receive::<PrepareRequest>()
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    let witness = match request {
        PrepareRequest::Witness(witness) => witness,
        _ => {
            return ServePrepareError::UnexpectedRequest.fail().spot(here!());
        }
    };

    // The verification of `witness` is delegated to the caller `witnessed_batch(..)`,
    // which validates the `WitnessedBatch` it builds from `witness`

    Ok(witness)
}
