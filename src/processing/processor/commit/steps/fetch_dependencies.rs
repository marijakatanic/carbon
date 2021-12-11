use buckets::{Buckets, Split};

use crate::{
    account::{Entry, Operation},
    commit::WitnessedBatch,
    database::{
        self,
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
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: &WitnessedBatch,
) -> Result<Vec<Option<Operation>>, Top<ServeCommitError>> {
    let dependencies = batch
        .payloads()
        .iter()
        .map(|payload| payload.dependency())
        .collect::<Vec<_>>();

    let database_dependencies = {
        let dependencies = dependencies.iter().cloned().flatten().collect::<Split<_>>();

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

    let mut database_dependencies = database_dependencies.into_iter();

    if missing_ids.is_empty() {
        let dependencies = dependencies
            .iter()
            .map(|dependency| {
                if dependency.is_some() {
                    Some(database_dependencies.next().unwrap().unwrap())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        return Ok(dependencies);
    }

    session
        .send(&CommitResponse::MissingDependencies(missing_ids.clone()))
        .await
        .pot(ServeCommitError::ConnectionError, here!())?;

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

    if completions.len() != missing_ids.len() {
        return ServeCommitError::MalformedDependencies.fail().spot(here!());
    }

    missing_ids
        .par_iter()
        .zip(completions.par_iter())
        .map(|(id, completion)| {
            if completion.id() != *id {
                ServeCommitError::MismatchedDependency.fail().spot(here!())
            } else {
                completion
                    .validate(discovery)
                    .pot(ServeCommitError::InvalidDependency, here!())
            }
        })
        .collect::<Result<_, _>>()?;

    let mut completions = completions.into_iter();

    let dependencies = dependencies
        .iter()
        .map(|dependency| {
            if dependency.is_some() {
                match database_dependencies.next().unwrap() {
                    Ok(dependency) => Some(dependency),
                    Err(_) => Some(completions.next().unwrap().operation().clone()),
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(dependencies)
}
