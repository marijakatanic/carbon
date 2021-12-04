use crate::{
    crypto::Identify,
    database::Database,
    discovery::Client,
    prepare::{SignedBatch, WitnessedBatch},
    processing::{
        messages::PrepareRequest,
        processor::prepare::{errors::ServePrepareError, steps},
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn witnessed_batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    session: &mut Session,
) -> Result<WitnessedBatch, Top<ServePrepareError>> {
    // Receive a `Vector<Prepare>`

    let request = session
        .receive::<PrepareRequest>()
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    let prepares = match request {
        PrepareRequest::Prepares(batch) => batch,
        _ => return ServePrepareError::UnexpectedRequest.fail().spot(here!()),
    };

    // Receive either:
    // - A witness, required to directly assemble a `WitnessedBatch`
    // - A collection of signatures required to assemble a `SignedBatch`,
    //   which will be validated to generate a witness shard

    let request = session
        .receive::<PrepareRequest>()
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    // Attain `WitnessedBatch`

    let batch = match request {
        PrepareRequest::Witness(witness) => {
            // A witness is sufficient to assemble a `WitnessedBatch`
            // (a plurality of other replicas verified the batch)
            Ok(WitnessedBatch::new(view.identifier(), prepares, witness))
        }
        PrepareRequest::Signatures(reduction_signature, individual_signatures) => {
            // Use signatures to obtain a `SignedBatch`
            let batch = SignedBatch::new(prepares, reduction_signature, individual_signatures);

            // Validate `batch` to obtain a witness shard
            let witness_shard =
                steps::validate_signed(keychain, discovery, database, session, &batch).await?;

            // Trade `witness_shard` for a full witness (which aggregates the witness shards
            // of a plurality of replicas in `view`)
            let witness = steps::trade_witnesses(view, session, &batch, witness_shard).await?;

            // Use `witness` to promote `batch` to `WitnessedBatch`
            let batch = batch.into_witnessed(view.identifier(), witness);

            Ok(batch)
        }
        _ => ServePrepareError::UnexpectedRequest.fail().spot(here!()),
    }?;

    // Validate and return `batch`

    batch
        .validate(discovery)
        .pot(ServePrepareError::InvalidBatch, here!())?;

    Ok(batch)
}
