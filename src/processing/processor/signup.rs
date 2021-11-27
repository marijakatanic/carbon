use crate::{
    crypto::Identify,
    database::Database,
    processing::{
        messages::{SignupRequest, SignupResponse},
        Processor,
    },
    signup::{IdAllocation, IdAssignment, IdClaim, IdRequest},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use rand::{self, seq::IteratorRandom};

use std::{iter, sync::Arc};

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, Identity, KeyChain},
    net::{Listener, Session, SessionListener},
    sync::{fuse::Fuse, voidable::Voidable},
};

use zebra::database::CollectionTransaction;

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
    #[doom(description("Foreign allocator"))]
    ForeignAllocator,
}

impl Processor {
    pub(in crate::processing) async fn signup<L>(
        keychain: KeyChain,
        view: View,
        database: Arc<Voidable<Database>>,
        listener: L,
    ) where
        L: Listener,
    {
        let mut listener = SessionListener::new(listener);
        let fuse = Fuse::new();

        loop {
            let (_, session) = listener.accept().await;

            let keychain = keychain.clone();
            let view = view.clone();
            let database = database.clone();

            fuse.spawn(async move {
                let _ = serve(keychain, view, database, session).await;
            });
        }
    }
}

async fn serve(
    keychain: KeyChain,
    view: View,
    database: Arc<Voidable<Database>>,
    mut session: Session,
) -> Result<(), Top<ServeSignupError>> {
    let identity = keychain.keycard().identity();

    let request = session
        .receive::<SignupRequest>()
        .await
        .pot(ServeSignupError::ConnectionError, here!())?;

    let response = {
        let mut database = database
            .lock()
            .pot(ServeSignupError::DatabaseVoid, here!())?;

        match request {
            SignupRequest::IdRequests(requests) => {
                id_requests_response(&keychain, identity, &view, &mut database, requests)?
            }

            SignupRequest::IdClaims(claims) => {
                id_claims_response(&keychain, &view, &mut database, claims)?
            }
        }
    };

    session
        .send(&response)
        .await
        .pot(ServeSignupError::ConnectionError, here!())?;

    session.end();

    Ok(())
}

fn id_requests_response(
    keychain: &KeyChain,
    identity: Identity,
    view: &View,
    database: &mut Database,
    requests: Vec<IdRequest>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    let allocations = requests
        .into_iter()
        .map(|request| {
            if request.view() != view.identifier() {
                return ServeSignupError::ForeignView.fail().spot(here!());
            }

            if request.allocator() != identity {
                return ServeSignupError::ForeignAllocator.fail().spot(here!());
            }

            request
                .validate()
                .pot(ServeSignupError::InvalidRequest, here!())?;

            Ok(allocate_id(identity, &keychain, &view, database, request))
        })
        .collect::<Result<Vec<_>, Top<ServeSignupError>>>()?;

    Ok(SignupResponse::IdAllocations(allocations))
}

fn id_claims_response(
    keychain: &KeyChain,
    view: &View,
    database: &mut Database,
    claims: Vec<IdClaim>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    let mut transaction = CollectionTransaction::new();

    let signatures = claims
        .into_iter()
        .map(|claim| {
            if claim.view() != view.identifier() {
                return ServeSignupError::ForeignView.fail().spot(here!());
            }

            claim
                .validate()
                .pot(ServeSignupError::InvalidRequest, here!())?;

            let stored = database
                .signup
                .claims
                .entry(claim.id())
                .or_insert(claim.clone());

            if stored.client() == claim.client() {
                // Double-inserts are harmless
                let _ = transaction.insert(claim.id());
                Ok(Ok(IdAssignment::certify(&keychain, &claim)))
            } else {
                Ok(Err(stored.clone())) // Already claimed by another identity
            }
        })
        .collect::<Result<Vec<Result<MultiSignature, IdClaim>>, Top<ServeSignupError>>>();

    // In order to keep `claims` in sync with `claimed`, `transaction` is
    // executed before bailing (if `signatures` is `Err`)
    database.signup.claimed.execute(transaction);
    Ok(SignupResponse::IdAssignments(signatures?))
}

fn allocate_id(
    identity: Identity,
    keychain: &KeyChain,
    view: &View,
    database: &mut Database,
    request: IdRequest,
) -> IdAllocation {
    if let Some(id) = database
        .signup
        .allocations
        .get(&request.client().identity())
    {
        return IdAllocation::new(&keychain, &request, *id);
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

        // `database.signup.allocated` contains all `Id`s the local replica has assigned in
        // the current view; because it is state-transferred, `database.signup.claims` contains
        // all `Id`s for which an `IdAssignment` has been generated in a past view. As a result,
        // every `Id` in `database.assignments` is necessarily in either `allocated` or `claims`:
        // if the `IdAssignment` was collected in this view, then necessarily its `Id` is in
        // `allocated` (as the local replica was the one that allocated the `Id`); if the
        // `IdAssignment` was collected in a previous view, then necessarily its `Id` is in
        // `claims` (due to the properties of state-transfer with a quorum of past members).
        if !database.signup.claims.contains_key(&id) && !database.signup.allocated.contains(&id) {
            break id;
        }
    };

    database.signup.allocated.insert(id);

    database
        .signup
        .allocations
        .insert(request.client().identity(), id);

    IdAllocation::new(&keychain, &request, id)
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    use crate::{discovery::Client, processing::test::System};

    #[tokio::test]
    async fn priority() {
        let System {
            view,
            brokers,
            processors,
        } = System::setup(4, 1).await;

        let allocator_identity = processors[0].0.keycard().identity();

        let client_keychain = KeyChain::random();
        let request = IdRequest::new(&client_keychain, &view, allocator_identity);

        let response = brokers[0].id_requests(vec![request.clone()]).await;

        let allocation = match response {
            SignupResponse::IdAllocations(mut allocations) => {
                assert_eq!(allocations.len(), 1);
                allocations.remove(0)
            }
            _ => panic!("unexpected response"),
        };

        allocation.validate(&request).unwrap();
        assert!(allocation.id() <= u32::MAX as u64);
    }

    #[tokio::test]
    async fn full_signup() {
        let System {
            view,
            brokers,
            processors,
        } = System::setup(4, 1).await;

        let allocator_identity = processors[0].0.keycard().identity();

        let client_keychain = KeyChain::random();
        let request = IdRequest::new(&client_keychain, &view, allocator_identity);

        let assignments = brokers[0].signup(vec![request.clone()]).await.unwrap();

        let client = Client::new(view, (Ipv4Addr::LOCALHOST, 0), Default::default());

        for assignment in assignments {
            assignment.unwrap().validate(&client).unwrap()
        }
    }
}
