use std::time::Instant;

use crate::{
    database::Database,
    discovery::Client,
    prepare::Prepare,
    processing::{
        messages::PrepareResponse,
        processor::prepare::{errors::ServePrepareError, steps},
    },
    view::View,
};

use doomstack::{here, ResultExt, Top};

use log::info;
use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

use zebra::vector::Vector;

pub(in crate::processing::processor::prepare) async fn batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    mut session: Session,
    prepares: Vector<Prepare>,
) -> Result<(), Top<ServePrepareError>> {
    // Obtain a `WitnessedBatch`

    let batch =
        steps::witnessed_batch(keychain, discovery, view, database, &mut session, prepares).await?;

    // Apply `batch` to `database` to obtain a `BatchCommitShard`

    let start = Instant::now();
    let shard = steps::apply_batch(keychain, view, database, batch).await?;
    info!(
        "Prepare: applied batch in {} ms",
        start.elapsed().as_millis()
    );

    // Send `shard` and end `session`

    session
        .send(&PrepareResponse::CommitShard(shard))
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    session.end();

    Ok(())
}
