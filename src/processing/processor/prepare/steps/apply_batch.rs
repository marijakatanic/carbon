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

    // This stores the `Equivocation`s produced by the elements of `batch`
    let mut exceptions = Vec::new();

    for (index, prepare) in batch.prepares().iter().enumerate() {
        // Build `PrepareHandle` relevant to `prepare`
        let handle = PrepareHandle::Batched {
            batch: batch.root(),
            index,
        };

        let state = match database.prepare.states.get(&prepare.id()) {
            Some(state) => match state {
                State::Consistent {
                    height: state_height,
                    commitment: state_commitment,
                    handle: state_handle,
                } => {
                    if prepare.height() == *state_height {
                        // A `Prepare` for this `prepare.height()` was previously received.

                        if prepare.commitment() == *state_commitment {
                            // `prepare` does not collide with the previously observed `Prepare`:
                            // `prepare` is valid, and no further update is required

                            continue;
                        } else {
                            // `prepare` collides with a previously observed `Prepare`:
                            // retrieve `Extract` to prove `Equivocation`
                            let state_extract = match state_handle {
                                PrepareHandle::Batched { batch, index } => {
                                    // `batch` is still in `database`, obtain `Extract` from there
                                    database.prepare.batches.get(batch).unwrap().extract(*index)
                                }

                                // The batch was garbage collected, leaving a ready-made `Extract` behind
                                PrepareHandle::Standalone(extract) => extract.clone(),
                            };

                            // Obtain conflicting `Extract` from `batch`, build `Equivocation`
                            let extract = batch.extract(index);
                            let equivocation = Equivocation::new(extract, state_extract);

                            // State must be updated to reflect the equivocation
                            State::Equivocated(equivocation)
                        }
                    } else {
                        // No `Prepare` was previously observed for this height: initialize
                        // the state to `Consistent`.

                        // (*) Remark: currently, no further check is performed on `prepare.height()`.
                        // In the future, a proof will be optionally provided by the broker to
                        // prove that the client successfully reached `prepare.height() - 1`.
                        //
                        // As a result, the following should apply:
                        //  - If `prepare.height()` is greater than both the highest observed
                        //    `Commit` for `prepare.id()` AND `state_height`, then the `state`
                        //    should be updated as done below.
                        //  - Otherwise, a higher `Commit` should be provided to the broker
                        //    as evidence of misbehaviour / delay, and `prepare.id()` should
                        //    be represented in `exceptions`.

                        State::Consistent {
                            height: prepare.height(),
                            commitment: prepare.commitment(),
                            handle,
                        }
                    }
                }

                // `State::Equivocated` is absorbing and must not be updated
                equivocated => equivocated.clone(),
            },
            None => State::Consistent {
                // No `Prepare` was previously observed for this height: initialize
                // the state to `Consistent`

                // Remark: see above (*)
                height: prepare.height(),
                commitment: prepare.commitment(),
                handle,
            },
        };

        // If `state` is `Equivocated`, add the relevant `Equivocation` to `exceptions`
        if let State::Equivocated(equivocation) = &state {
            exceptions.push(equivocation.clone());
        }

        // Update `states`, flag new state in `stale` to allow efficient
        // flushing to `advertisements` (performed immediately before
        // state transfer)
        database.prepare.states.insert(prepare.id(), state);
        database.prepare.stale.insert(prepare.id());
    }

    // Store `batch` in `batches`

    let root = batch.root();
    let holder = BatchHolder::new(batch);

    database.prepare.batches.insert(root, holder);

    // Use `exceptions` to return an appropriate `BatchCommitShard`

    let shard = BatchCommitShard::new(&keychain, view.identifier(), root, exceptions);

    Ok(shard)
}
