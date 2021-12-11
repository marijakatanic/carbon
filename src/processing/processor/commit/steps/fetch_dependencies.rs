use buckets::{Buckets, Split};

use crate::{
    account::Entry,
    commit::WitnessedBatch,
    database::{
        commit::{BatchHolder, PayloadHandle},
        Database,
    },
    discovery::Client,
    processing::{messages::CommitResponse, processor::commit::errors::ServeCommitError},
};

use doomstack::{here, ResultExt, Top};

use std::collections::HashMap;

use talk::{
    crypto::{
        primitives::{hash::Hash, multi::Signature as MultiSignature},
        KeyChain,
    },
    net::Session,
    sync::voidable::Voidable,
};

pub(in crate::processing::processor::commit) async fn fetch_dependencies(
    _keychain: &KeyChain,
    _discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: &WitnessedBatch,
) -> Result<MultiSignature, Top<ServeCommitError>> {
    let dependencies = batch
        .payloads()
        .iter()
        .map(|payload| payload.dependency())
        .flatten()
        .collect::<Split<_>>();

    let database_dependencies = {
        let mut database = database
            .lock()
            .pot(ServeCommitError::DatabaseVoid, here!())?;

        fn fields(
            database: &mut Database,
        ) -> (
            &mut Buckets<HashMap<Entry, PayloadHandle>>,
            &HashMap<Hash, BatchHolder>,
        ) {
            (&mut database.commit.payloads, &database.commit.batches)
        }

        let (payloads, batches) = fields(&mut database);

        buckets::apply_attached(
            payloads,
            batches,
            dependencies,
            |payloads, batches, dependency| {
                match payloads.get(&dependency) {
                    Some(handle) => {
                        let holder = batches.get(&handle.batch).unwrap();

                        if holder.completed() {
                            return Ok(holder.batch().payloads()[handle.index].operation().clone());
                        }
                    }
                    None => {}
                }

                Err(dependency.id)
            },
        )
    }
    .join();

    let missing_ids = database_dependencies
        .iter()
        .filter_map(|dependency| dependency.as_ref().err().cloned())
        .collect::<Vec<_>>();

    session
        .send(&CommitResponse::MissingDependencies(missing_ids))
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    todo!()
}
