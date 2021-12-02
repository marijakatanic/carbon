use doomstack::Doom;

#[derive(Doom)]
pub(in crate::processing::processor::prepare) enum ServePrepareError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Unexpected request"))]
    UnexpectedRequest,
    #[doom(description("Malformed batch"))]
    MalformedBatch,
    #[doom(description("Database void"))]
    DatabaseVoid,
    #[doom(description("Malformed id assignments"))]
    MalformedIdAssignments,
    #[doom(description("Invalid id assignment"))]
    InvalidIdAssignment,
    #[doom(description("Invalid batch"))]
    InvalidBatch,
    #[doom(description("Invalid witness"))]
    InvalidWitness,
}
