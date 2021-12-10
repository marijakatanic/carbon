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
    #[doom(description("Motions overflow"))]
    MotionsOverflow,
    #[doom(description("Double support"))]
    DoubleSupport,
    #[doom(description("Unexpected abandon"))]
    UnexpectedAbandon,
}
