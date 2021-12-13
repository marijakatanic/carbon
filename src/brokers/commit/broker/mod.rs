use crate::{
    crypto::Identify,
    data::{PingBoard, Sponge},
    discovery::Client,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{net::SocketAddr, sync::Arc};

use talk::{
    link::context::ConnectDispatcher,
    net::{Connector, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    net::{TcpListener, ToSocketAddrs},
};

pub(crate) struct Broker {
    address: SocketAddr,
    _fuse: Fuse,
}

#[derive(Doom)]
pub(crate) enum BrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

impl Broker {
    pub async fn new<A, C>(
        discovery: Arc<Client>,
        view: View,
        address: A,
        connector: C,
    ) -> Result<Self, Top<BrokerError>>
    where
        A: ToSocketAddrs,
        C: Connector,
    {
        let listener = TcpListener::bind(address)
            .await
            .map_err(BrokerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let address = listener
            .local_addr()
            .map_err(BrokerError::initialize_failed)
            .map_err(Doom::into_top)
            .spot(here!())?;

        let dispatcher = ConnectDispatcher::new(connector);
        let context = format!("{:?}::processor::commit", view.identifier());
        let connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let brokerage_sponge = Arc::new(Sponge::new(Default::default())); // TODO: Add settings
        let ping_board = PingBoard::new(&view);

        let fuse = Fuse::new();

        {
            let discovery = discovery.clone();
            let brokerage_sponge = brokerage_sponge.clone();

            fuse.spawn(async move {
                Broker::listen(discovery, brokerage_sponge, listener).await;
            });
        }

        {
            let view = view.clone();
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move {
                Broker::flush(view, brokerage_sponge, ping_board, connector).await;
            });
        }

        for replica in view.members().keys().copied() {
            let ping_board = ping_board.clone();
            let connector = connector.clone();

            fuse.spawn(async move { Broker::ping(ping_board, connector, replica).await });
        }

        Ok(Broker {
            address,
            _fuse: fuse,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

mod broker;
mod flush;
mod frontend;
mod orchestrate;
mod ping;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        account::{Entry, Operation},
        brokers::{
            commit::{BrokerFailure, Request},
            prepare::{
                BrokerFailure as PrepareBrokerFailure, Inclusion as PrepareInclusion,
                Request as PrepareRequest,
            },
            signup::BrokerFailure as SignupBrokerFailure,
            test::System,
        },
        commit::{Commit, CommitProof, Completion, CompletionProof, Payload},
        prepare::BatchCommit,
        signup::{IdAssignment, IdRequest, SignupSettings},
    };

    use talk::{crypto::KeyChain, net::PlainConnection};

    use tokio::net::TcpStream;

    #[tokio::test]
    async fn develop() {
        let System {
            view,
            discovery_server: _discovery_server,
            discovery_client: _discovery_client,
            processors,
            mut signup_brokers,
            mut prepare_brokers,
            mut commit_brokers,
        } = System::setup(4, 1, 1, 1).await;

        let client_keychain = KeyChain::random();

        // Brokers

        let signup_broker = signup_brokers.remove(0);
        let prepare_broker = prepare_brokers.remove(0);
        let commit_broker = commit_brokers.remove(0);

        // Signup

        let allocator_identity = processors[0].0.keycard().identity();

        let request = IdRequest::new(
            &client_keychain,
            &view,
            allocator_identity,
            SignupSettings::default().work_difficulty,
        );

        let stream = TcpStream::connect(signup_broker.address()).await.unwrap();
        let mut connection: PlainConnection = stream.into();

        connection.send(&request).await.unwrap();

        let assignment = connection
            .receive::<Result<IdAssignment, SignupBrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        println!("Signup completed.");

        // --------------------- Withdraw ---------------------

        let payload = Payload::new(
            Entry {
                id: assignment.id(),
                height: 1,
            },
            Operation::withdraw(assignment.id(), 0, 0),
        );

        let prepare = payload.prepare();

        // Prepare

        let request = PrepareRequest::new(
            &client_keychain,
            assignment.clone(),
            prepare.height(),
            prepare.commitment(),
        );

        let stream = TcpStream::connect(prepare_broker.address()).await.unwrap();
        let mut connection: PlainConnection = stream.into();

        connection.send(&request).await.unwrap();

        let inclusion = connection
            .receive::<Result<PrepareInclusion, PrepareBrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        let reduction_shard = inclusion
            .certify_reduction(&client_keychain, request.prepare())
            .unwrap();

        connection.send(&reduction_shard).await.unwrap();

        let batch_commit = connection
            .receive::<Result<BatchCommit, PrepareBrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        let commit_proof = CommitProof::new(batch_commit, inclusion.proof);

        let commit = Commit::new(commit_proof, payload.clone());

        println!("[Withdraw] Prepare completed.");

        // Commit

        let request = Request::new(commit, None);

        let stream = TcpStream::connect(commit_broker.address()).await.unwrap();
        let mut connection: PlainConnection = stream.into();

        connection.send(&request).await.unwrap();

        let completion_proof = connection
            .receive::<Result<CompletionProof, BrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        let withdrawal = Completion::new(completion_proof, payload);

        println!("[Withdraw] Commit completed.");

        // --------------------- Deposit ---------------------

        let payload = Payload::new(
            Entry {
                id: assignment.id(),
                height: 2,
            },
            Operation::deposit(withdrawal.entry(), None, true),
        );

        let prepare = payload.prepare();

        // Prepare

        let request = PrepareRequest::new(
            &client_keychain,
            assignment.clone(),
            prepare.height(),
            prepare.commitment(),
        );

        let stream = TcpStream::connect(prepare_broker.address()).await.unwrap();
        let mut connection: PlainConnection = stream.into();

        connection.send(&request).await.unwrap();

        let inclusion = connection
            .receive::<Result<PrepareInclusion, PrepareBrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        let reduction_shard = inclusion
            .certify_reduction(&client_keychain, request.prepare())
            .unwrap();

        connection.send(&reduction_shard).await.unwrap();

        let batch_commit = connection
            .receive::<Result<BatchCommit, PrepareBrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        let commit_proof = CommitProof::new(batch_commit, inclusion.proof);

        let commit = Commit::new(commit_proof, payload.clone());

        println!("[Deposit] Prepare completed.");

        // Commit

        let request = Request::new(commit, Some(withdrawal.clone()));

        let stream = TcpStream::connect(commit_broker.address()).await.unwrap();
        let mut connection: PlainConnection = stream.into();

        connection.send(&request).await.unwrap();

        let completion_proof = connection
            .receive::<Result<CompletionProof, BrokerFailure>>()
            .await
            .unwrap()
            .unwrap();

        println!("[Deposit] Completion proof:\n{:?}", completion_proof);

        let deposit = Completion::new(completion_proof, payload);

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
