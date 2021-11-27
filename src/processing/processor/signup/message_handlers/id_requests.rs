use crate::{
    crypto::Identify,
    database::Database,
    processing::{messages::SignupResponse, processor::signup::errors::ServeSignupError},
    signup::{IdAllocation, IdRequest},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use rand::{self, seq::IteratorRandom};

use std::iter;

use talk::crypto::{Identity, KeyChain};

pub(in crate::processing::processor::signup) fn id_requests(
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
