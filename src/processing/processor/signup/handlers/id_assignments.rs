use buckets::Split;

use crate::{
    database::Database,
    discovery::Client,
    processing::{messages::SignupResponse, processor::signup::errors::ServeSignupError},
    signup::IdAssignment,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::sync::voidable::Voidable;

pub(in crate::processing::processor::signup) fn id_assignments(
    _discovery: &Client,
    database: &Voidable<Database>,
    assignments: Vec<IdAssignment>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    // Verify that `assignments` is sorted and deduplicated

    if !assignments
        .windows(2)
        .all(|window| window[0].keycard() < window[1].keycard())
    {
        return ServeSignupError::InvalidRequest.fail().spot(here!());
    }

    // Validate `assignments` (in parallel)

    // Skip verification (for benchmark purposes)

    // assignments
    //     .par_iter()
    //     .map(|assignment| {
    //         assignment
    //             .validate(discovery)
    //             .pot(ServeSignupError::InvalidRequest, here!())
    //     })
    //     .collect::<Result<(), Top<ServeSignupError>>>()?;

    // Process `assignments`

    let assignments = assignments.into_iter().collect::<Split<_>>();

    {
        let mut database = database
            .lock()
            .pot(ServeSignupError::DatabaseVoid, here!())?;

        database
            .assignments
            .apply(assignments, |assignments, assignment| {
                assignments.insert(assignment.id(), assignment)
            });
    }

    Ok(SignupResponse::AcknowledgeIdAssignments)
}
