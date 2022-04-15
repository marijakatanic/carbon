use std::time::Instant;

use crate::{
    commit::{Payload, WitnessedBatch},
    crypto::Identify,
    database::Database,
    discovery::Client,
    processing::{
        messages::CommitRequest,
        processor::commit::{errors::ServeCommitError, steps},
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use log::info;

use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

use zebra::vector::Vector;

pub(in crate::processing::processor::commit) async fn witnessed_batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    session: &mut Session,
    payloads: Vector<Payload>,
) -> Result<WitnessedBatch, Top<ServeCommitError>> {
    // Receive either:
    //  - A witness
    //  - A request to validate the batch and produce a witness
    //    shard, which will be traded for a witness

    let request = session
        .receive::<CommitRequest>()
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    let witness = match request {
        CommitRequest::Witness(witness) => {
            // The batch was verified by a plurality of other replicas
            // in `view`, no check on the batch is needed
            Ok(witness)
        }
        CommitRequest::WitnessRequest => {
            // Validate the batch to obtain a witness shard
            let start = Instant::now();
            let witness_shard =
                steps::validate_batch(keychain, discovery, database, session, &payloads).await?;
            info!("Commit: validated batch in {} ms", start.elapsed().as_millis());

            // Trade `witness_shard` for a full witness (which aggregates the witness shards
            // of a plurality of replicas in `view`)
            let witness = steps::trade_witnesses(session, witness_shard).await?;

            Ok(witness)
        }
        _ => ServeCommitError::UnexpectedRequest.fail().spot(here!()),
    }?;

    // Assemble `payloads` and `witness` in a `WitnessedBatch` to validate and return

    let batch = WitnessedBatch::new(view.identifier(), payloads, witness);

    batch
        .validate(discovery)
        .pot(ServeCommitError::InvalidBatch, here!())?;

    Ok(batch)
}
