use crate::{
    commit::Payload,
    database::Database,
    discovery::Client,
    processing::{
        messages::CommitResponse,
        processor::commit::{errors::ServeCommitError, steps},
    },
    view::View,
};

use doomstack::Top;

use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

use zebra::vector::Vector;

pub(in crate::processing::processor::commit) async fn batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    mut session: Session,
    payloads: Vector<Payload>,
) -> Result<(), Top<ServeCommitError>> {
    let batch =
        steps::witnessed_batch(keychain, discovery, view, database, &mut session, payloads).await?;

    let dependencies = steps::fetch_dependencies(discovery, database, &mut session, &batch).await?;

    let shard = steps::apply_batch(
        keychain,
        discovery,
        view,
        database,
        &mut session,
        batch,
        dependencies,
    )
    .await?;

    session
        .send(&CommitResponse::CompletionShard(shard))
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    session.end();

    Ok(())
}
