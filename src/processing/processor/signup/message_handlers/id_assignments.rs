use crate::{
    database::Database,
    discovery::Client,
    processing::{messages::SignupResponse, processor::signup::errors::ServeSignupError},
    signup::IdAssignment,
};

use doomstack::Top;

pub(in crate::processing::processor::signup) fn id_assignments(
    _discovery: &Client,
    _database: &mut Database,
    _assignments: Vec<IdAssignment>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    todo!()
}
