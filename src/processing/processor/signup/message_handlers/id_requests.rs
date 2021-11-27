use crate::{
    crypto::Identify,
    database::Database,
    processing::{
        messages::SignupResponse, processor::signup::errors::ServeSignupError,
        processor_settings::SignupSettings,
    },
    signup::{IdAllocation, IdRequest},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use rand::{self, seq::IteratorRandom};

use std::iter;

use talk::crypto::{Identity, KeyChain};

pub(in crate::processing::processor::signup) fn id_requests(
    keychain: &KeyChain,
    view: &View,
    database: &mut Database,
    requests: Vec<IdRequest>,
    settings: &SignupSettings,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    let identity = keychain.keycard().identity();

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
                .validate(settings.work_difficulty)
                .pot(ServeSignupError::InvalidRequest, here!())?;

            Ok(allocate_id(
                &keychain, identity, &view, database, request, settings,
            ))
        })
        .collect::<Result<Vec<_>, Top<ServeSignupError>>>()?;

    Ok(SignupResponse::IdAllocations(allocations))
}

fn allocate_id(
    keychain: &KeyChain,
    identity: Identity,
    view: &View,
    database: &mut Database,
    request: IdRequest,
    settings: &SignupSettings,
) -> IdAllocation {
    if let Some(id) = database
        .signup
        .allocations
        .get(&request.client().identity())
    {
        // `request` was previously served, repeat previous `IdAllocation`
        return IdAllocation::new(&keychain, &request, *id);
    }

    let full_range = view.allocation_range(identity);

    let priority_available = full_range.start == 0;
    let priority_range = 0..(u32::MAX as u64);

    // If `priority_available`, try picking from `priority_range` first, then expand to `full_range`
    // after a given number of attempts (this happens with higher probability as `priority_range`
    // progressively saturates)
    let mut ranges = iter::repeat(priority_range)
        .take(if priority_available {
            settings.priority_attempts
        } else {
            0
        })
        .chain(iter::repeat(full_range));

    let id = loop {
        let id = ranges
            .next()
            .unwrap()
            .choose(&mut rand::thread_rng())
            .unwrap();

        // The following hold true:
        //  - `database.signup.claims` contains all `Id`s for which an `IdAssignment` has been
        //    generated in a past view. This is because `claims` are state-transferred.
        //  - `database.signup.allocated` contains all `Id`s the local replica has assigned in `view`
        //
        // As a result, every `Id` for which an `IdAssignment` has been generated is necessarily
        // in `allocated` union `claims`:
        //  - If the `IdAssignment` was collected in a previous view, then necessarily its `Id` is in
        // `claims` (due to the properties of state-transfer with a quorum of past members, see above).
        //  - If the `IdAssignment` was collected in `view`, then necessarily its `Id` is in
        //    `allocated` (as the local replica was the one that allocated the `Id`)
        //
        // Remark: the above does not guarantee that `id` will be successfully claimed by `client`
        // (if that was the case, consensus would be solved deterministically and asynchronously).
        // Indeed, `id` might have been allocated in a previous view then only partially claimed.
        // The local replica might be, e.g., the only one not to have gathered `id` in `claimed`
        // upon state transfer. Upon seeing conflicting claims, all other replicas would then
        // reject `client`'s claim of `id`.
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
