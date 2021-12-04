use crate::{
    crypto::Certificate,
    prepare::{SignedBatch, WitnessStatement},
    processing::{
        messages::{PrepareRequest, PrepareResponse},
        processor::prepare::errors::ServePrepareError,
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::primitives::multi::Signature as MultiSignature, net::Session};

pub(in crate::processing::processor::prepare) async fn trade_witnesses(
    view: &View,
    session: &mut Session,
    batch: &SignedBatch,
    shard: MultiSignature,
) -> Result<Certificate, Top<ServePrepareError>> {
    session
        .send(&PrepareResponse::WitnessShard(shard))
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

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

    let witness_statement = WitnessStatement::new(batch.root());

    witness
        .verify_plurality(&view, &witness_statement)
        .pot(ServePrepareError::InvalidWitness, here!())?;

    Ok(witness)
}
