use buckets::{Buckets, Split};

use crate::{
    account::{Entry, Operation},
    commit::{Payload, WitnessedBatch},
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

pub(in crate::processing::processor::commit) async fn fetch_dependencies(
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: &WitnessedBatch,
) -> Result<Vec<Option<Operation>>, Top<ServeCommitError>> {
    // Collect all completed `Operation`s in `database` on which
    // the `Payloads` of `batch` depend

    let database_operations = {
        // Collect `Id` and dependency `Entry` of all `Payload`s in `batch` whose dependency is `Some`.
        let queries = batch.payloads().iter().filter_map(|payload| {
            payload
                .dependency()
                .map(|dependency| (payload.id(), dependency))
        });

        // `Split` `queries` according to the `Id` of each dependency (`database`
        // is not for the `Payload`s in `batch`, but for their dependencies)
        let split = Split::with_key(queries, |(_, dependency)| dependency.id);

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

        // The following maps each element `(id, dependency)` of `split` onto:
        //  - `Ok(operation)`, if an appropriate `Operation` `operation` could be found for
        //    `dependency` from `database`
        //  - `Err(id, dependency)`, meaning that `session` must be queried for a `Completion`
        //    for the dependency `dependency` of `id`'s payload
        buckets::apply_attached(
            payloads,
            batches,
            split,
            |payloads, batches, (id, dependency)| {
                // An `Operation` is available for `dependency` in `database` only if:
                //  - An entry exists for `dependency` in `payloads`;
                //  - Such entry is member of a batch to which a `BatchCompletion` was attached
                //    that does not except `dependency.id`
                match payloads.get(&dependency) {
                    Some(handle) => {
                        // No `BatchHolder` can be left dangling after garbage
                        // collection: the following always succeeds
                        let holder = batches.get(&handle.batch).unwrap();

                        if let Some(completion) = holder.completion() {
                            // Technically, if `completion` excepts `dependency.id`,
                            // then no `Completion` will ever be gathered for `dependency`.
                            // This means that the broker is Byzantine, and will fail
                            // to exhibit a `Completion` for `dependency` when asked
                            // to do so.
                            if !completion.excepts(dependency.id) {
                                return Ok(holder.batch().payloads()[handle.index]
                                    .operation()
                                    .clone());
                            }
                        }
                    }
                    None => {}
                }

                Err((id, dependency))
            },
        )
    }
    .join();

    // Extract `Option` dependencies from `batch.payloads()`

    let dependencies = batch.payloads().iter().map(Payload::dependency);

    // Extract `Id`s and corresponding `Entry` dependencies missing from `database`

    let missing = database_operations
        .iter()
        .filter_map(|operation| operation.as_ref().err().cloned())
        .collect::<Vec<_>>();

    let mut database_operations = database_operations.into_iter();

    if missing.is_empty() {
        // No query to `session` is necessary: `unwrap` `database_operations` to match
        // each `Some` element of `dependencies`
        let operations = dependencies
            .map(|dependency| {
                dependency.map(|_| {
                    // The following `unwrap`s cannot fail:
                    // - There are as many `database_operations` as there are `Some` `dependencies`
                    // - All elements of `database_operations` are guaranteed to be `Ok`
                    database_operations.next().unwrap().unwrap()
                })
            })
            .collect::<Vec<_>>();

        return Ok(operations);
    }

    // Send the `MissingDependencies` vector of `Id`s of payloads whose dependency is `Some`,
    // but could not be satisfied in `database`

    let missing_ids = missing.iter().map(|(id, _)| *id).collect::<Vec<_>>();

    session
        .send(&CommitResponse::MissingDependencies(missing_ids))
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    // Receive `Dependencies` (any other request is unexpected)

    let request = session
        .receive::<CommitRequest>()
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

    let completions = match request {
        CommitRequest::Dependencies(completions) => completions,
        _ => {
            return ServeCommitError::UnexpectedRequest.fail().spot(here!());
        }
    };

    // Each element of `completions` must match  a corresponding element of `missing`

    if completions.len() != missing.len() {
        return ServeCommitError::MalformedDependencies.fail().spot(here!());
    }

    // Validate each element of `completions` against the corresponding element of `missing`

    missing
        .par_iter()
        .zip(completions.par_iter())
        .map(|((_, dependency), completion)| {
            // `completion` must be relevant to `dependency` and valid
            if completion.entry() != *dependency {
                ServeCommitError::MismatchedDependency.fail().spot(here!())
            } else {
                completion
                    .validate(discovery)
                    .pot(ServeCommitError::InvalidDependency, here!())
            }
        })
        .collect::<Result<_, _>>()?;

    // Use `completions` to fill the gaps in `database_operations`
    // and satisfy every element of `dependencies`

    let mut session_operations = completions
        .into_iter()
        .map(|completion| completion.operation().clone());

    let operations = dependencies
        .map(|dependency| {
            dependency.map(|_| {
                // The next element of `database_operations` is `Some`
                // if and only if the ignored argument `_` of this closure
                // could be found in `database`.
                // If the next element of `database_operations` is `None`,
                // then `_` was satisfied by querying `session`, and the
                // relevant operation can be extracted from the next element
                // of `completions`.
                match database_operations.next().unwrap() {
                    Ok(operation) => operation,
                    // Noting that `session_operations` is as long as
                    // `missing`, the following cannot fail
                    Err(_) => session_operations.next().unwrap(),
                }
            })
        })
        .collect::<Vec<_>>();

    Ok(operations)
}
