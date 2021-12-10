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
    let request = session
        .receive::<CommitRequest>()
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    let witness = match request {
        CommitRequest::Witness(witness) => Ok(witness),
        CommitRequest::WitnessRequest => {
            let witness_shard =
                steps::validate_batch(keychain, discovery, database, session, &payloads).await?;

            let witness = steps::trade_witnesses(session, witness_shard).await?;

            Ok(witness)
        }
        _ => ServeCommitError::UnexpectedRequest.fail().spot(here!()),
    }?;

    let batch = WitnessedBatch::new(view.identifier(), payloads, witness);

    batch
        .validate(discovery)
        .pot(ServeCommitError::InvalidBatch, here!())?;

    Ok(batch)
}
