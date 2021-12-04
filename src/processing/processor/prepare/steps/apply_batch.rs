use crate::{
    crypto::Identify,
    database::{
        prepare::{BatchHolder, PrepareHandle, State},
        Database,
    },
    prepare::{BatchCommitShard, Equivocation, WitnessedBatch},
    processing::processor::prepare::errors::ServePrepareError,
    view::View,
};

use doomstack::{here, ResultExt, Top};

use talk::{crypto::KeyChain, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn apply_batch(
    keychain: &KeyChain,
    view: &View,
    database: &Voidable<Database>,
    batch: WitnessedBatch,
) -> Result<BatchCommitShard, Top<ServePrepareError>> {
    let mut database = database
        .lock()
        .pot(ServePrepareError::DatabaseVoid, here!())?;

    let mut exceptions = Vec::new();

    for (index, prepare) in batch.prepares().iter().enumerate() {
        let id = prepare.id();
        let height = prepare.height();
        let commitment = prepare.commitment();
        let prepare = PrepareHandle::Batched {
            batch: batch.root(),
            index,
        };

        let state = match database.prepare.states.get(&id) {
            Some(state) => match state {
                State::Consistent {
                    height: state_height,
                    commitment: state_commitment,
                    prepare: state_prepare,
                } => {
                    if height == *state_height {
                        if commitment == *state_commitment {
                            continue;
                        } else {
                            let state_extract = match state_prepare {
                                PrepareHandle::Batched {
                                    batch: state_batch,
                                    index: state_index,
                                } => database
                                    .prepare
                                    .batches
                                    .get(state_batch)
                                    .unwrap()
                                    .extract(*state_index),
                                PrepareHandle::Standalone(extract) => extract.clone(),
                            };

                            let equivocation =
                                Equivocation::new(batch.extract(index), state_extract);

                            State::Equivocated(equivocation)
                        }
                    } else {
                        State::Consistent {
                            height,
                            commitment,
                            prepare,
                        }
                    }
                }
                equivocated => equivocated.clone(),
            },
            None => State::Consistent {
                height,
                commitment,
                prepare,
            },
        };

        if let State::Equivocated(equivocation) = &state {
            exceptions.push(equivocation.clone());
        }

        database.prepare.states.insert(id, state);
        database.prepare.stale.insert(id);
    }

    let root = batch.root();
    let holder = BatchHolder::new(batch);

    database.prepare.batches.insert(root, holder);

    let shard = BatchCommitShard::new(&keychain, view.identifier(), root, exceptions);

    Ok(shard)
}
