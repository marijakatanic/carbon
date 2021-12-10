use doomstack::Doom;

#[derive(Doom)]
pub(crate) enum OperationError {
    #[doom(description("Overdraft"))]
    Overdraft,
    #[doom(description("Unexpected dependency"))]
    UnexpectedDependency,
    #[doom(description("Illegitimate deposit"))]
    IllegitimateDeposit,
    #[doom(description("Exclusion invalid"))]
    ExclusionInvalid,
    #[doom(description("Double deposit"))]
    DoubleDeposit,
}
