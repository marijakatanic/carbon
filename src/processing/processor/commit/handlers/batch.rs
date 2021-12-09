use crate::{
    commit::Payload,
    database::Database,
    discovery::Client,
    processing::processor::commit::{errors::ServeCommitError, steps},
    view::View,
};

use doomstack::Top;

use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::commit) async fn batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    mut session: Session,
    payloads: Vec<Payload>,
) -> Result<(), Top<ServeCommitError>> {
    // Obtain a `WitnessedBatch`

    let _batch =
        steps::witnessed_batch(keychain, discovery, view, database, &mut session, payloads).await?;

    todo!()
}
