use crate::{
    crypto::Identify,
    database::Database,
    processing::{Processor, SignupRequest, SignupResponse},
    signup::{IdAllocation, IdRequest},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use rand::{self, seq::IteratorRandom};

use std::{iter, sync::Arc};

use talk::{
    crypto::{Identity, KeyChain},
    net::{Listener, SecureConnection},
    sync::{fuse::Fuse, voidable::Voidable},
};

#[derive(Doom)]
enum ServeSignupError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Database void"))]
    DatabaseVoid,
    #[doom(description("Invalid request"))]
    InvalidRequest,
    #[doom(description("Foreign view"))]
    ForeignView,
    #[doom(description("Foreign assigner"))]
    ForeignAssigner,
}

impl Processor {
    pub(in crate::processing) async fn signup<L>(
        keychain: KeyChain,
        view: View,
        database: Arc<Voidable<Database>>,
        mut listener: L,
    ) where
        L: Listener,
    {
        let fuse = Fuse::new();

        loop {
            if let Ok((_, connection)) = listener.accept().await {
                let keychain = keychain.clone();
                let view = view.clone();
                let database = database.clone();

                fuse.spawn(async move {
                    let _ = Processor::serve_signup(keychain, view, database, connection).await;
                });
            }
        }
    }

    async fn serve_signup(
        keychain: KeyChain,
        view: View,
        database: Arc<Voidable<Database>>,
        mut connection: SecureConnection,
    ) -> Result<(), Top<ServeSignupError>> {
        let identity = keychain.keycard().identity();

        loop {
            let request = connection
                .receive::<SignupRequest>()
                .await
                .pot(ServeSignupError::ConnectionError, here!())?;

            let response = {
                let mut database = database
                    .lock()
                    .pot(ServeSignupError::DatabaseVoid, here!())?;

                match request {
                    SignupRequest::IdRequests(requests) => {
                        let allocations = requests
                            .into_iter()
                            .map(|request| {
                                request
                                    .validate()
                                    .pot(ServeSignupError::InvalidRequest, here!())?;

                                if request.view() != view.identifier() {
                                    return ServeSignupError::ForeignView.fail().spot(here!());
                                }

                                if request.assigner() != identity {
                                    return ServeSignupError::ForeignAssigner.fail().spot(here!());
                                }

                                Ok(Processor::allocate_id(
                                    identity,
                                    &keychain,
                                    &view,
                                    &mut database,
                                    request,
                                ))
                            })
                            .collect::<Result<Vec<_>, Top<ServeSignupError>>>()?;

                        SignupResponse::IdAllocations(allocations)
                    }
                }
            };

            connection
                .send(&response)
                .await
                .pot(ServeSignupError::ConnectionError, here!())?;
        }
    }

    fn allocate_id(
        identity: Identity,
        keychain: &KeyChain,
        view: &View,
        database: &mut Database,
        request: IdRequest,
    ) -> IdAllocation {
        if let Some(allocation) = database.signup.assignments.get(&request.identity()) {
            return allocation.clone();
        }

        let full_range = view.allocation_range(identity);

        let priority_available = full_range.start == 0;
        let priority_range = 0..(u32::MAX as u64);

        let mut ranges = iter::repeat(priority_range)
            .take(if priority_available { 30 } else { 0 }) // TODO: Add settings
            .chain(iter::repeat(full_range));

        let id = loop {
            let id = ranges
                .next()
                .unwrap()
                .choose(&mut rand::thread_rng())
                .unwrap();

            if !database.keycards.contains_key(&id) && !database.signup.assigned.contains(&id) {
                break id;
            }
        };

        let allocation = IdAllocation::new(&keychain, &request, id);

        database
            .signup
            .assignments
            .insert(request.identity(), allocation.clone());

        database.signup.assigned.insert(id);

        allocation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::processing::test::System;

    #[tokio::test]
    async fn priority() {
        let System {
            view,
            brokers,
            processors,
        } = System::setup(4, 1).await;

        let assigner_identity = processors[0].0.keycard().identity();

        let client_keychain = KeyChain::random();
        let id_request = IdRequest::new(&client_keychain, &view, assigner_identity);

        let response = brokers[0].id_requests(vec![id_request.clone()]).await;

        let id_allocation = match response {
            SignupResponse::IdAllocations(mut allocations) => {
                assert_eq!(allocations.len(), 1);
                allocations.remove(0)
            } // _ => panic!("unexpected response"),
        };

        id_allocation.validate(&id_request).unwrap();
        assert!(id_allocation.id() <= u32::MAX as u64);
    }
}
