use buckets::Split;

use crate::{
    database::Database,
    discovery::Client,
    prepare::{Prepare, SignedBatch},
    processing::{
        messages::{PrepareRequest, PrepareResponse},
        processor::prepare::errors::ServePrepareError,
    },
};

use doomstack::{here, Doom, ResultExt, Top};

use rayon::prelude::*;

use talk::{crypto::KeyCard, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn fetch_keycards(
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: &SignedBatch,
) -> Result<Vec<KeyCard>, Top<ServePrepareError>> {
    // For each element of `batch.prepares()`, retrieve from `database`,
    // if available, the `KeyCard` corresponding to the relevant `Id`

    let ids = Split::with_key(batch.prepares().iter().map(Prepare::id), |id| *id);

    let database_keycards = {
        let mut database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        // Map each `id` in `ids` into:
        //  - `Ok(keycard)`, if some `keycard` is available for `id`
        //    in `database.assignments`
        //  - `Err(id)` otherwise, signifying that an `IdAssignment`
        //    for `id` must be obtained before `batch` can be validated
        database
            .assignments
            .apply(ids, |assignments, id| match assignments.get(&id) {
                Some(assignment) => Ok(assignment.keycard().clone()),
                None => Err(id),
            })
    }
    .join();

    // Extract all unknown `Id`s from `database_keycards`'s `Err` elements

    let unknown_ids = database_keycards
        .iter()
        .map(Result::as_ref)
        .filter_map(Result::err)
        .copied()
        .collect::<Vec<_>>();

    // If `unknown_ids` is empty, no further communication is required

    if unknown_ids.is_empty() {
        // Because `unknown_ids` is empty, all `database_keycards` are `Ok`,
        // and can be safely unwrapped
        let keycards = database_keycards
            .into_iter()
            .map(|keycard| keycard.unwrap())
            .collect::<Vec<_>>();

        return Ok(keycards);
    }

    // Query for `unknown_ids`

    session
        .send(&PrepareResponse::UnknownIds(unknown_ids.clone())) // TODO: Remove unnecessary `clone`
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    // Receive requested `IdAssignments`

    let request = session
        .receive::<PrepareRequest>()
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    let assignments = match request {
        PrepareRequest::Assignments(id_assignments) => id_assignments,
        _ => {
            return ServePrepareError::UnexpectedRequest.fail().spot(here!());
        }
    };

    // Validate `id_assignments` against `unknown_ids`

    // This check is necessary to ensure that the subsequent `zip` will
    // iterate fully over both `unknown_ids` and `assignments`
    if assignments.len() != unknown_ids.len() {
        return ServePrepareError::MalformedIdAssignments
            .fail()
            .spot(here!());
    }

    // Check that each element `assignments` is valid and relevant to the
    // corresponding element of `unknown_ids`
    unknown_ids
        .par_iter()
        .zip(assignments.par_iter())
        .map(|(id, assignment)| {
            if assignment.id() != *id {
                ServePrepareError::MismatchedIdAssignment
                    .fail()
                    .spot(here!())
            } else {
                assignment
                    .validate(discovery)
                    .pot(ServePrepareError::InvalidIdAssignment, here!())
            }
        })
        .collect::<Result<_, _>>()?;

    // Store `assignments` in `database`, retain only the `KeyCard`s
    // necessary to fill the gaps in `database_keycards`

    let assignments = assignments.into_iter().collect::<Split<_>>();

    let mut missing_keycards = {
        let mut database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        // The following collects all `KeyCard`s in a `Vec` to avoid
        // lingering references to `database` (which needs to be
        // unlocked in a timely fashion)
        database
            .assignments
            .apply(assignments, |assignments, assignment| {
                let keycard = assignment.keycard().clone();
                assignments.insert(assignment.id(), assignment);
                keycard
            })
    }
    .join()
    .into_iter(); // Elements will be extracted in order from `missing_keycards`

    // Use `missing_keycards` to fill the gaps in `database_keycards`

    let keycards = database_keycards
        .into_iter()
        .map(|keycard| match keycard {
            Ok(keycard) => keycard,
            // Because `missing_keycards.len() == id_assignments.len() == unknown_ids.len()`,
            // `missing_keycards.next()` is guaranteed to be `Some`
            Err(_) => missing_keycards.next().unwrap(),
        })
        .collect::<Vec<_>>();

    // Each element of `keycards` now cointains the `KeyCard` relevant
    // to the corresponding element of `batch.prepares()`

    Ok(keycards)
}
