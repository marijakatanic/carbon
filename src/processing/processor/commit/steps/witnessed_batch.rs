use crate::{
    commit::{Payload, WitnessedBatch},
    crypto::Identify,
    database::Database,
    discovery::Client,
    processing::{messages::CommitRequest, processor::commit::errors::ServeCommitError},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

use zebra::vector::Vector;

pub(in crate::processing::processor::commit) async fn witnessed_batch(
    _keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    _database: &Voidable<Database>,
    session: &mut Session,
    payloads: Vector<Payload>,
) -> Result<WitnessedBatch, Top<ServeCommitError>> {
    let request = session
        .receive::<CommitRequest>()
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    let batch = match request {
        CommitRequest::Witness(witness) => {
            Ok(WitnessedBatch::new(view.identifier(), payloads, witness))
        }
        CommitRequest::WitnessRequest => {
            todo!()
        }
        _ => ServeCommitError::UnexpectedRequest.fail().spot(here!()),
    }?;

    batch
        .validate(discovery)
        .pot(ServeCommitError::InvalidBatch, here!())?;

    todo!()
}
