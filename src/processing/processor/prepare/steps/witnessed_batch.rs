use std::time::Instant;

use crate::{
    crypto::Identify,
    database::Database,
    discovery::Client,
    prepare::{Prepare, SignedBatch, WitnessedBatch},
    processing::{
        messages::PrepareRequest,
        processor::prepare::{errors::ServePrepareError, steps},
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use log::info;
use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};
use zebra::vector::Vector;

pub(in crate::processing::processor::prepare) async fn witnessed_batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    session: &mut Session,
    prepares: Vector<Prepare>,
) -> Result<WitnessedBatch, Top<ServePrepareError>> {
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

            let start = Instant::now();
            // Validate `batch` to obtain a witness shard
            let witness_shard =
                steps::validate_signed(keychain, discovery, database, session, &batch).await?;
            info!("Validated batch in {} ms", start.elapsed().as_millis());

            // Trade `witness_shard` for a full witness (which aggregates the witness shards
            // of a plurality of replicas in `view`)
            let witness = steps::trade_witnesses(session, witness_shard).await?;

            // Use `witness` to promote `batch` to `WitnessedBatch`
            let batch = batch.into_witnessed(view.identifier(), witness);

            Ok(batch)
        }
        _ => ServePrepareError::UnexpectedRequest.fail().spot(here!()),
    }?;

    // Validate and return `batch` (this checks the correctness of the `witness`es acquired above)
    let start = Instant::now();
    batch
        .validate(discovery)
        .pot(ServePrepareError::InvalidBatch, here!())?;
    info!("Validated witness in {} ms", start.elapsed().as_millis());

    Ok(batch)
}
