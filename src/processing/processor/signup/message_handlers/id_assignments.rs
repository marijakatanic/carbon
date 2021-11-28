use crate::{
    database::Database,
    discovery::Client,
    processing::{messages::SignupResponse, processor::signup::errors::ServeSignupError},
    signup::IdAssignment,
};

use doomstack::{here, ResultExt, Top};

pub(in crate::processing::processor::signup) fn id_assignments(
    discovery: &Client,
    database: &mut Database,
    assignments: Vec<IdAssignment>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    for assignment in assignments {
        assignment
            .validate(discovery)
            .pot(ServeSignupError::InvalidRequest, here!())?;

        database.assignments.insert(assignment.id(), assignment);
    }

    Ok(SignupResponse::AcknowledgeIdAssignments)
}
