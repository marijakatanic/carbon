use buckets::{Buckets, Split};
use log::warn;

use crate::{
    account::Id,
    commit::{Payload, WitnessStatement},
    database::{
        prepare::{BatchHolder, PrepareHandle, State},
        Database,
    },
    discovery::Client,
    prepare::Prepare,
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

use zebra::vector::Vector;

pub(in crate::processing::processor::commit) async fn validate_batch(
    keychain: &KeyChain,
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    payloads: &Vector<Payload>,
) -> Result<MultiSignature, Top<ServeCommitError>> {
    // Verify that `paylods` is strictly increasing by `Id`
    // (this ensures searchability of `Id`s)

    if !payloads
        .items()
        .windows(2)
        .all(|window| window[0].id() < window[1].id())
    {
        return ServeCommitError::MalformedBatch.fail().spot(here!());
    }

    // Obtain the `Prepare` relevant to each element of `payloads`

    let prepares = payloads
        .items()
        .iter()
        .map(Payload::prepare)
        .collect::<Split<_>>();

    // Use `database` to filter all cached, committed `Prepare`s out of `prepares`

    let unproven_prepares = {
        let mut database = database
            .lock()
            .pot(ServeCommitError::DatabaseVoid, here!())?;

        fn fields(
            database: &mut Database,
        ) -> (
            &mut Buckets<HashMap<Id, State>>,
            &HashMap<Hash, BatchHolder>,
        ) {
            (&mut database.prepare.states, &database.prepare.batches)
        }

        let (states, batches) = fields(&mut database);

        buckets::apply_sparse_attached(states, batches, prepares, |states, batches, prepare| {
            // Check if:
            // - A `Consistent` entry exists in `states` for `prepare.id()`;
            // - The height and commitment of such entry match `prepare.id()`
            //   and `prepare.commitment()`, meaning that `prepare` was
            //   the last `Prepare` seen by the local replica for `prepare.id()`,
            //   and `prepare` was not previously committed;
            // - Such entry belongs to a `prepare::WitnessedBatch` for which
            //   a `BatchCommit` was obtained that does not except `prepare.id()`.
            // If so, no additional information is necessary to verify that
            // `prepare` (and consequently the correspondingly revealed `Payload`)
            // is ready to be committed: `prepare` can be filtered out from `prepares`.

            match states.get(&prepare.id()) {
                // State relevant to `prepare.id()` must exist
                Some(state) => match state {
                    // `state` must be consistent
                    State::Consistent {
                        height,
                        commitment,
                        handle,
                    } => {
                        // `state`'s `height` and `commitment` must match `prepare`'s
                        if *height == prepare.height() && *commitment == prepare.commitment() {
                            match handle {
                                // `handle` must be `Batched` (committed batches are garbage
                                // collected along with their `BatchCommit`s)
                                PrepareHandle::Batched { batch, .. } => {
                                    // No `BatchHolder` can be left dangling after garbage
                                    // collection: the following always succeeds
                                    let holder = batches.get(batch).unwrap();

                                    // A `BatchCommit` must be attached to `holder`
                                    if let Some(commit) = holder.commit() {
                                        // `commit` must not except `prepare.id()`
                                        if !commit.excepts(prepare.id()) {
                                            // `prepare` can be safely filtered out of `prepares`:
                                            // its corresponding payload can be committed
                                            return None;
                                        }
                                    }
                                }
                                PrepareHandle::Standalone(_) => (),
                            }
                        }
                    }
                    State::Equivocated(_) => (),
                },
                None => (),
            }

            Some(prepare)
        })
    };

    // All elements of `unproven_prepares` necessitate a `CommitProof`
    // in order for their corresponding payloads to be committed
    if !unproven_prepares.is_empty() {
        warn!("Have unproven prepares!");

        // Query `session` for the `Id` corresponding to each
        // element of `missing_proofs`

        let missing_proofs = unproven_prepares
            .iter()
            .map(Prepare::id)
            .collect::<Vec<_>>();

        session
            .send(&CommitResponse::MissingCommitProofs(missing_proofs))
            .await
            .pot(ServeCommitError::ConnectionError, here!())?;

        // Receive `CommitProofs` (any other request is unexpected)

        let request = session
            .receive::<CommitRequest>()
            .await
            .pot(ServeCommitError::ConnectionError, here!())?;

        let proofs = match request {
            CommitRequest::CommitProofs(proofs) => Ok(proofs),
            _ => ServeCommitError::UnexpectedRequest.fail().spot(here!()),
        }?;

        // Each element of `proofs` must match  a corresponding element of `unproven_prepares`
        if proofs.len() != unproven_prepares.len() {
            return ServeCommitError::MalformedCommitProofs.fail().spot(here!());
        }

        // Each element of `unproven_prepares` must be valid against its corresponding element of `proofs`
        unproven_prepares
            .into_par_iter()
            .zip(proofs.into_par_iter())
            .map(|(prepare, proof)| {
                proof
                    .validate(discovery, &prepare)
                    .pot(ServeCommitError::InvalidCommitProof, here!())
            })
            .collect::<Result<(), Top<ServeCommitError>>>()?;
    }

    // All `payloads` are eligible to be committed: sign and return a witness shard

    let witness_statement = WitnessStatement::new(payloads.root());
    let witness_shard = keychain.multisign(&witness_statement).unwrap();

    Ok(witness_shard)
}
