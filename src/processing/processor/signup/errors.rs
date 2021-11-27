use doomstack::Doom;

#[derive(Doom)]
pub(in crate::processing::processor::signup) enum ServeSignupError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Database void"))]
    DatabaseVoid,
    #[doom(description("Invalid request"))]
    InvalidRequest,
    #[doom(description("Foreign view"))]
    ForeignView,
    #[doom(description("Foreign allocator"))]
    ForeignAllocator,
}
