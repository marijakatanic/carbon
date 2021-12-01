use crate::{
    brokers::prepare::{BrokerSettings, Failure, Inclusion, Request},
    crypto::Identify,
    data::{Sponge, SpongeSettings},
    discovery::Client,
    prepare::{Batch, ReductionStatement},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use std::{iter, net::SocketAddr, sync::Arc};

use talk::{
    crypto::primitives::multi::Signature as MultiSignature,
    link::context::ConnectDispatcher,
    net::{Connector, PlainConnection, SessionConnector},
    sync::fuse::Fuse,
};

use tokio::{
    io,
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot::{self, Receiver, Sender},
};

use zebra::vector::Vector;

type ReductionInlet = Sender<Result<Reduction, Failure>>;
type ReductionOutlet = Receiver<Result<Reduction, Failure>>;

pub(crate) struct Broker {
    address: SocketAddr,
    _fuse: Fuse,
}

struct Brokerage {
    request: Request,
    reduction_inlet: ReductionInlet,
}

struct Reduction {
    index: usize,
    inclusion: Inclusion,
    reduction_sponge: Arc<Sponge<(usize, MultiSignature)>>,
}

#[derive(Doom)]
pub(crate) enum BrokerError {
    #[doom(description("Failed to initialize broker: {}", source))]
    #[doom(wrap(initialize_failed))]
    InitializeFailed { source: io::Error },
}

#[derive(Doom)]
pub(crate) enum ServeError {
    #[doom(description("Connection error"))]
    ConnectionError,
    #[doom(description("Request invalid"))]
    RequestInvalid,
    #[doom(description("`Brokerage` forfeited (most likely, the `Broker` is shutting down)"))]
    #[doom(wrap(request_forfeited))]
    BrokerageForfeited { source: oneshot::error::RecvError },
    #[doom(description("Root shard invalid"))]
    RootShardInvalid,
}

impl Broker {
    pub async fn new<A, C>(
        discovery: Client,
        view: View,
        address: A,
        connector: C,
        settings: BrokerSettings,
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
        let context = format!("{:?}::processor::prepare", view.identifier());
        let _connector = Arc::new(SessionConnector::new(dispatcher.register(context)));

        let discovery = Arc::new(discovery);
        let brokerage_sponge = Arc::new(Sponge::new(settings.brokerage_sponge_settings));

        let fuse = Fuse::new();

        {
            let brokerage_sponge = brokerage_sponge.clone();

            fuse.spawn(async move {
                Broker::listen(discovery, brokerage_sponge, listener).await;
            });
        }

        let settings = settings.reduction_sponge_settings;
        fuse.spawn(async move {
            Broker::flush(brokerage_sponge, settings).await;
        });

        Ok(Broker {
            address,
            _fuse: fuse,
        })
    }

    async fn listen(
        discovery: Arc<Client>,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
        listener: TcpListener,
    ) {
        let fuse = Fuse::new();

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let connection: PlainConnection = stream.into();

                let discovery = discovery.clone();
                let brokerage_sponge = brokerage_sponge.clone();

                fuse.spawn(async move {
                    let _ = Broker::serve(connection, discovery, brokerage_sponge).await;
                });
            }
        }
    }

    async fn serve(
        mut connection: PlainConnection,
        discovery: Arc<Client>,
        brokerage_sponge: Arc<Sponge<Brokerage>>,
    ) -> Result<(), Top<ServeError>> {
        let request = connection
            .receive::<Request>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        request
            .validate(discovery.as_ref())
            .pot(ServeError::RequestInvalid, here!())?;

        let keycard = request.keycard();

        let (reduction_inlet, reduction_outlet) = oneshot::channel();

        let brokerage = Brokerage {
            request,
            reduction_inlet,
        };

        brokerage_sponge.push(brokerage);

        let reduction = reduction_outlet
            .await
            .map_err(ServeError::request_forfeited)
            .map_err(Doom::into_top)
            .spot(here!())?;

        if let Err(failure) = reduction {
            connection
                .send::<Result<Inclusion, Failure>>(&Err(failure))
                .await
                .pot(ServeError::ConnectionError, here!())?;

            return Ok(());
        }

        let Reduction {
            index,
            inclusion,
            reduction_sponge,
        } = reduction.unwrap();

        let root = inclusion.root();

        connection
            .send::<Result<Inclusion, Failure>>(&Ok(inclusion))
            .await
            .pot(ServeError::ConnectionError, here!())?;

        let reduction_shard = connection
            .receive::<MultiSignature>()
            .await
            .pot(ServeError::ConnectionError, here!())?;

        reduction_shard
            .verify([&keycard], &ReductionStatement::new(root))
            .pot(ServeError::RootShardInvalid, here!())?;

        let _ = reduction_sponge.push((index, reduction_shard));

        // TODO: Wait for and forward outcome to client
        todo!()
    }

    async fn flush(brokerage_sponge: Arc<Sponge<Brokerage>>, settings: SpongeSettings) {
        let fuse = Fuse::new();

        loop {
            let brokerages = Broker::prepare(brokerage_sponge.flush().await);

            if brokerages.is_empty() {
                continue;
            }

            let settings = settings.clone();

            fuse.spawn(async move {
                Broker::broker(brokerages, settings).await;
            });
        }
    }

    fn prepare(mut brokerages: Vec<Brokerage>) -> Vec<Brokerage> {
        // Sort `brokerages` by requestor

        brokerages.sort_by_key(|brokerage| brokerage.request.id());

        // Deduplicate and fail `brokerages` by requestor

        // The following implementation does not use `Vec::dedup_*` because,
        // in order to fail a duplicate `Brokerage`, it needs to consume
        // its `outcome_inlet` (which mutable references don't allow)
        let mut previous = None;

        brokerages
            .into_iter()
            .filter_map(|brokerage| {
                if Some(brokerage.request.id()) == previous {
                    let _ = brokerage.reduction_inlet.send(Err(Failure::Throttle));
                    None
                } else {
                    previous = Some(brokerage.request.id());
                    Some(brokerage)
                }
            })
            .collect()
    }

    async fn broker(brokerages: Vec<Brokerage>, settings: SpongeSettings) {
        let mut assignments = Vec::new();
        let mut prepares = Vec::new();
        let mut individual_signatures = Vec::new();

        let mut reduction_inlets = Vec::new();

        for Brokerage {
            request:
                Request {
                    assignment,
                    prepare,
                    signature,
                },
            reduction_inlet,
        } in brokerages
        {
            assignments.push(assignment);
            prepares.push(prepare);
            individual_signatures.push(Some(signature));

            reduction_inlets.push(reduction_inlet)
        }

        let prepares = Vector::new(prepares).unwrap();
        let inclusions = Inclusion::batch(&prepares);

        let reduction_sponge = Arc::new(Sponge::<(usize, MultiSignature)>::new(settings));

        let reductions = inclusions
            .into_iter()
            .zip(iter::repeat(reduction_sponge.clone()))
            .enumerate()
            .map(|(index, (inclusion, reduction_sponge))| Reduction {
                index,
                inclusion,
                reduction_sponge,
            })
            .collect::<Vec<_>>();

        for (reduction, reduction_inlet) in reductions.into_iter().zip(reduction_inlets) {
            let _ = reduction_inlet.send(Ok(reduction));
        }

        let reductions = reduction_sponge.flush().await;

        let root_signature =
            MultiSignature::aggregate(reductions.into_iter().map(|(index, shard)| {
                individual_signatures[index] = None;
                shard
            }))
            .unwrap();

        let _batch = Batch::new(prepares, root_signature, individual_signatures);
    }
}
