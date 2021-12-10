use buckets::{Buckets, Split};

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
    let prepares = payloads
        .items()
        .iter()
        .map(Payload::prepare)
        .collect::<Split<_>>();

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
            match states.get(&prepare.id()) {
                Some(state) => match state {
                    State::Consistent {
                        height,
                        commitment,
                        handle,
                    } => {
                        if *height == prepare.height() && *commitment == prepare.commitment() {
                            match handle {
                                PrepareHandle::Batched { batch, .. } => {
                                    let holder = batches.get(batch).unwrap(); // TODO: Check this `unwrap`

                                    if holder.committed() {
                                        return None;
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

    if !unproven_prepares.is_empty() {
        let missing_proofs = unproven_prepares
            .iter()
            .map(Prepare::id)
            .collect::<Vec<_>>();

        session
            .send(&CommitResponse::MissingProofs(missing_proofs))
            .await
            .pot(ServeCommitError::ConnectionError, here!())?;

        let request = session
            .receive::<CommitRequest>()
            .await
            .pot(ServeCommitError::ConnectionError, here!())?;

        let proofs = match request {
            CommitRequest::CommitProofs(proofs) => Ok(proofs),
            _ => ServeCommitError::UnexpectedRequest.fail().spot(here!()),
        }?;

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

    let witness_statement = WitnessStatement::new(payloads.root());
    let witness_shard = keychain.multisign(&witness_statement).unwrap();

    Ok(witness_shard)
}
