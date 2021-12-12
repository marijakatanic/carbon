use crate::{
    account::Id,
    commit::{Commit, Completion},
    discovery::Client,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Request {
    commit: Commit,
    dependency: Option<Completion>,
}

#[derive(Doom)]
pub(crate) enum RequestError {
    #[doom(description("`Commit` invalid"))]
    CommitInvalid,
    #[doom(description("Dependency mismatch"))]
    DependencyMismatch,
    #[doom(description("Dependency invalid"))]
    DependencyInvalid,
}

impl Request {
    pub fn new(commit: Commit, dependency: Option<Completion>) -> Self {
        Request { commit, dependency }
    }

    pub fn id(&self) -> Id {
        self.commit.payload().id()
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<RequestError>> {
        self.commit
            .validate(discovery)
            .pot(RequestError::CommitInvalid, here!())?;

        match (self.commit.operation().dependency(), &self.dependency) {
            (Some(dependency), Some(completion)) => {
                if completion.entry() != dependency {
                    return RequestError::DependencyMismatch.fail().spot(here!());
                }

                completion
                    .validate(discovery)
                    .pot(RequestError::DependencyInvalid, here!())?;
            }

            (None, None) => {}

            _ => {
                return RequestError::DependencyMismatch.fail().spot(here!());
            }
        }

        Ok(())
    }
}
