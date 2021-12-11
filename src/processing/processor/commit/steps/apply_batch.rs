use buckets::{Buckets, Split};

use crate::{
    account::{Account, Entry, Operation},
    commit::{BatchCompletionShard, Payload, WitnessedBatch},
    database::{
        commit::{BatchHolder, PayloadHandle},
        Database,
    },
    discovery::Client,
    processing::{
        messages::{CommitRequest, CommitResponse},
        processor::commit::errors::ServeCommitError,
    },
};

use doomstack::{here, Doom, ResultExt, Top};

use rayon::prelude::*;

use std::collections::HashMap;

use talk::{crypto::primitives::hash::Hash, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::commit) async fn apply_batch(
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: WitnessedBatch,
    dependencies: Vec<Option<Operation>>,
) -> Result<BatchCompletionShard, Top<ServeCommitError>> {
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

    let split = Split::with_key(
        batch
            .payloads()
            .iter()
            .cloned()
            .zip(dependencies.iter().cloned()),
        |(payload, _)| payload.id(),
    );

    let exceptions = {
        let mut database = database
            .lock()
            .pot(ServeCommitError::DatabaseVoid, here!())?;

        let exceptions = buckets::apply_sparse(
            &mut database.accounts,
            split,
            |accounts, (payload, dependency)| {
                if accounts.get_mut(&payload.id()).unwrap().apply(
                    &payload,
                    dependency.as_ref(),
                    &Default::default(),
                ) {
                    // TODO: Add settings
                    None
                } else {
                    Some(payload.id())
                }
            },
        );
    };

    todo!()
}
