use crate::{
    database::Database,
    discovery::Client,
    processing::{messages::SignupResponse, processor::signup::errors::ServeSignupError},
    signup::IdAssignment,
};

use doomstack::{here, Doom, ResultExt, Top};

pub(in crate::processing::processor::signup) fn id_assignments(
    discovery: &Client,
    database: &mut Database,
    assignments: Vec<IdAssignment>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    // Verify that `assignments` is sorted and deduplicated

    if !assignments
        .windows(2)
        .all(|window| window[0].keycard() < window[1].keycard())
    {
        return ServeSignupError::InvalidRequest.fail().spot(here!());
    }

    // Process `assignments`

    for assignment in assignments {
        assignment
            .validate(discovery)
            .pot(ServeSignupError::InvalidRequest, here!())?;

        database.assignments.insert(assignment.id(), assignment);
    }

    Ok(SignupResponse::AcknowledgeIdAssignments)
}
