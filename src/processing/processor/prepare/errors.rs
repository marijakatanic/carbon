use doomstack::Doom;

#[derive(Doom)]
pub(in crate::processing::processor::prepare) enum ServePrepareError {
    #[doom(description("Connection error"))]
    ConnectionError,
}
