use crate::{
    database::Database,
    processing::{messages::SignupResponse, processor::signup::errors::ServeSignupError},
    signup::IdAssignment,
};

use doomstack::Top;

pub(in crate::processing::processor::signup) fn id_assignments(
    _database: &mut Database,
    _assignments: Vec<IdAssignment>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    todo!()
}
