use doomstack::Doom;

#[derive(Doom)]
pub(crate) enum OperationError {
    #[doom(description("Overdraft"))]
    Overdraft,
}
