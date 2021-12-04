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

use talk::{net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn gather_assignments(
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: &SignedBatch,
) -> Result<(), Top<ServePrepareError>> {
    // Identify unknown `Id`s in `batch`

    let unknown_ids = {
        let database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        batch
            .prepares()
            .iter()
            .map(|prepare| prepare.id())
            .filter(|id| !database.assignments.contains_key(&id))
            .collect::<Vec<_>>()
    };

    // If `unknown_ids` is empty, no further communication is required

    if unknown_ids.is_empty() {
        return Ok(());
    }

    // Query for `unknown_ids`

    session
        .send(&PrepareResponse::UnknownIds(unknown_ids.clone())) // TODO: Remove unnecessary `clone`
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    // Receive requested of `IdAssignments`

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
    // `Result<(), Top<ServePrepareError>>`, the following `collect` has no
    // memory footprint.
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

    // Store `id_assignments` in `database`

    {
        let mut database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        for id_assignment in id_assignments {
            database
                .assignments
                .insert(id_assignment.id(), id_assignment);
        }
    }

    Ok(())
}
