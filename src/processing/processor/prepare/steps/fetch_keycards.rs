use crate::{
    database::Database,
    discovery::Client,
    prepare::SignedBatch,
    processing::{
        messages::{PrepareRequest, PrepareResponse},
        processor::prepare::errors::ServePrepareError,
    },
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::KeyCard, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn fetch_keycards(
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: &SignedBatch,
) -> Result<Vec<KeyCard>, Top<ServePrepareError>> {
    // For each element of `batch.prepares()`, retrieve from `database`,
    // if available, the `KeyCard` corresponding to the relevant `Id`

    let database_keycards = {
        let database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        // Map each `prepare` in `batch.prepares()` into:
        //  - `Ok(keycard)`, if some `keycard` is available for `prepare.id()`
        //    in `database.assignments`
        //  - `Err(prepare.id())` otherwise, signifying that an `IdAssignment`
        //    for `prepare.id()` must be obtained before `batch` can be validated
        batch
            .prepares()
            .iter()
            .map(|prepare| match database.assignments.get(&prepare.id()) {
                Some(assignment) => Ok(assignment.keycard().clone()),
                None => Err(prepare.id()),
            })
            .collect::<Vec<_>>()
    };

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

    let id_assignments = match request {
        PrepareRequest::Assignments(id_assignments) => id_assignments,
        _ => {
            return ServePrepareError::UnexpectedRequest.fail().spot(here!());
        }
    };

    // Validate `id_assignments` against `unknown_ids`

    // This check is necessary to ensure that the subsequent `zip` will
    // iterate fully over both `unknown_ids` and `id_assignments`
    if id_assignments.len() != unknown_ids.len() {
        return ServePrepareError::MalformedIdAssignments
            .fail()
            .spot(here!());
    }

    // Check that each element `id_assignments` is valid and relevant to the
    // corresponding element of `unknown_ids`. Because the following collects
    // an iterator of `Result<(), Top<ServePrepareError>>` into a single
    // `Result<(), Top<ServePrepareError>>`, this check has no memory footprint.
    unknown_ids
        .iter()
        .zip(id_assignments.iter())
        .map(|(id, id_assignment)| {
            if id_assignment.id() != *id {
                ServePrepareError::MismatchedIdAssignment
                    .fail()
                    .spot(here!())
            } else {
                id_assignment
                    .validate(discovery)
                    .pot(ServePrepareError::InvalidIdAssignment, here!())
            }
        })
        .collect::<Result<_, _>>()?;

    // Store `id_assignments` in `database`, retain only the `KeyCard`s
    // necessary to fill the gaps in `database_keycards`

    let mut missing_keycards = {
        let mut database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        // The following collects all `KeyCard`s in a `Vec` to avoid
        // lingering references to `database` (which needs to be
        // unlocked in a timely fashion)
        id_assignments
            .into_iter()
            .map(|id_assignment| {
                let keycard = id_assignment.keycard().clone();

                database
                    .assignments
                    .insert(id_assignment.id(), id_assignment);

                keycard
            })
            .collect::<Vec<_>>()
    }
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
