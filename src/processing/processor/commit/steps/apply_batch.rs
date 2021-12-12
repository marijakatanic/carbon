use buckets::{Buckets, Split};

use crate::{
    account::{Account, Entry, Id, Operation},
    commit::{BatchCompletionShard, Payload, WitnessedBatch},
    crypto::Identify,
    database::{
        commit::{BatchHolder, PayloadHandle},
        Database,
    },
    processing::processor::commit::errors::ServeCommitError,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::collections::HashMap;

use talk::{crypto::KeyChain, sync::voidable::Voidable};

pub(in crate::processing::processor::commit) async fn apply_batch(
    keychain: &KeyChain,
    view: &View,
    database: &Voidable<Database>,
    batch: WitnessedBatch,
    dependencies: Vec<Option<Operation>>,
) -> Result<BatchCompletionShard, Top<ServeCommitError>> {
    let root = batch.root();

    // Check if `batch` can be applied to `database` (i.e.,
    // every `Entry` in `batch.payloads()` is applicable
    // to the relevant element of `database.accounts`)

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
            // Fetch `entry.id`'s `Account` (if no operation was previously
            // processed from `entry.id`, initialize an empty `Account`)

            let account = accounts
                .entry(entry.id)
                .or_insert_with(|| Account::new(entry.id, &Default::default())); // TODO: Add settings

            // If `entry.height` is not applicable to `account`, return `entry.id`

            if account.applicable(entry.height) {
                None
            } else {
                Some(entry.id)
            }
        })
    };

    // The whole batch must be applicable in order to be processed
    if !inapplicable_ids.is_empty() {
        return ServeCommitError::BatchInapplicable.fail().spot(here!());
    }

    // Zip together in a `Split` an enumeration of `payloads` and `dependencies`

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

        // Apply each `(_, (payload, dependency))` in `applications` to `database.accounts`,
        // then store `payload` in `database.commit.payloads` as a `PayloadHandle`

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
                // Apply `(payload, dependency)` to `accounts`

                // All missing accounts where created when checking applicability,
                // so the following `unwrap` is guaranteed to succeed
                let exception = if accounts.get_mut(&payload.id()).unwrap().apply(
                    &payload,
                    dependency.as_ref(),
                    &Default::default(), // TODO: Add settings
                ) {
                    None
                } else {
                    Some(payload.id())
                };

                // Store (a reference to) `payload` in `payloads`

                payloads.insert(
                    payload.entry(),
                    PayloadHandle {
                        batch: *root,
                        index,
                    },
                );

                exception
            },
        );

        // Store `batch` in `database.commit.batches`

        database
            .commit
            .batches
            .insert(root, BatchHolder::new(batch));

        exceptions
    };

    // Sign and return a `BatchCompletionShard` with the appropriate `exceptions`

    let shard = BatchCompletionShard::new(keychain, view.identifier(), root, exceptions);

    Ok(shard)
}
