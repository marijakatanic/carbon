use crate::{
    database::Database,
    processing::Processor,
    processing::{SignupRequest, SignupResponse},
    signup::{IdAllocation, IdRequest},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use rand;
use rand::seq::IteratorRandom;

use std::iter;
use std::sync::Arc;

use talk::crypto::{Identity, KeyChain};
use talk::net::{Listener, SecureConnection};
use talk::sync::fuse::Fuse;
use talk::sync::voidable::Voidable;

#[derive(Doom)]
enum ServeSignupError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Database void"))]
    DatabaseVoid,
    #[doom(description("Invalid request"))]
    InvalidRequest,
}

impl Processor {
    pub(in crate::processing) async fn signup<L>(
        keychain: Arc<KeyChain>,
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
        keychain: Arc<KeyChain>,
        view: View,
        database: Arc<Voidable<Database>>,
        mut connection: SecureConnection,
    ) -> Result<(), Top<ServeSignupError>> {
        let assigner = keychain.keycard().identity();

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
                                    .validate(&view, assigner)
                                    .pot(ServeSignupError::InvalidRequest, here!())?;

                                Ok(Processor::allocate_id(
                                    assigner,
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
        assigner: Identity,
        keychain: &KeyChain,
        view: &View,
        database: &mut Database,
        request: IdRequest,
    ) -> IdAllocation {
        if let Some(allocation) = database.signup.assignments.get(&request.identity()) {
            return allocation.clone();
        }

        let full_range = view.allocation_range(assigner);

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

        let allocation = IdAllocation::new(&keychain, &view, id, request.identity());

        database
            .signup
            .assignments
            .insert(request.identity(), allocation.clone());

        database.signup.assigned.insert(id);

        allocation
    }
}
