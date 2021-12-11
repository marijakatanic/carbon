use buckets::{Buckets, Split};

use crate::{
    account::{Account, Entry, Id, Operation},
    commit::{BatchCompletionShard, Payload, WitnessedBatch},
    crypto::Identify,
    database::{
        commit::{BatchHolder, PayloadHandle},
        Database,
    },
    discovery::Client,
    processing::{
        messages::{CommitRequest, CommitResponse},
        processor::commit::errors::ServeCommitError,
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use rayon::prelude::*;

use std::collections::HashMap;

use talk::{
    crypto::{primitives::hash::Hash, KeyChain},
    net::Session,
    sync::voidable::Voidable,
};

pub(in crate::processing::processor::commit) async fn apply_batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: WitnessedBatch,
    dependencies: Vec<Option<Operation>>,
) -> Result<BatchCompletionShard, Top<ServeCommitError>> {
    let root = batch.root();

    let entries = batch
        .payloads()
        .iter()
        .map(Payload::entry)
        .collect::<Split<_>>();

    let inapplicable_ids = {
        let mut database = database
            .lock()
            .pot(ServeCommitError::DatabaseVoid, here!())?;

        buckets::apply_sparse(&mut database.accounts, entries, |accounts, entry| {
            let account = accounts
                .entry(entry.id)
                .or_insert_with(|| Account::new(entry.id, &Default::default())); // TODO: Add settings

            if account.applicable(entry.height) {
                None
            } else {
                Some(entry.id)
            }
        })
    };

    if !inapplicable_ids.is_empty() {
        return ServeCommitError::BatchInapplicable.fail().spot(here!());
    }

    let applications = Split::with_key(
        batch
            .payloads()
            .iter()
            .cloned()
            .zip(dependencies.iter().cloned())
            .enumerate(),
        |(_, (payload, _))| payload.id(),
    );

    let exceptions = {
        let mut database = database
            .lock()
            .pot(ServeCommitError::DatabaseVoid, here!())?;

        fn fields(
            database: &mut Database,
        ) -> (
            &mut Buckets<HashMap<Id, Account>>,
            &mut Buckets<HashMap<Entry, PayloadHandle>>,
        ) {
            (&mut database.accounts, &mut database.commit.payloads)
        }

        let (accounts, payloads) = fields(&mut database);

        let exceptions = buckets::apply_sparse_attached(
            (accounts, payloads),
            &root,
            applications,
            |(accounts, payloads), root, (index, (payload, dependency))| {
                if accounts.get_mut(&payload.id()).unwrap().apply(
                    &payload,
                    dependency.as_ref(),
                    &Default::default(), // TODO: Add settings
                ) {
                    payloads.insert(
                        payload.entry(),
                        PayloadHandle {
                            batch: *root,
                            index,
                        },
                    );

                    None
                } else {
                    Some(payload.id())
                }
            },
        );

        database
            .commit
            .batches
            .insert(root, BatchHolder::new(batch));

        exceptions
    };

    let shard = BatchCompletionShard::new(keychain, view.identifier(), root, exceptions);

    Ok(shard)
}
