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

    if unknown_ids.is_empty() {
        return Ok(());
    }

    session
        .send(&PrepareResponse::UnknownIds(unknown_ids.clone())) // TODO: Remove unnecessary `clone`
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

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

    if id_assignments.len() != unknown_ids.len() {
        return ServePrepareError::MalformedIdAssignments
            .fail()
            .spot(here!());
    }

    if !unknown_ids
        .iter()
        .zip(id_assignments.iter())
        .all(|(id, id_assignment)| {
            id_assignment.id() == *id && id_assignment.validate(discovery).is_ok()
        })
    {
        return ServePrepareError::InvalidIdAssignment.fail().spot(here!());
    }

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
