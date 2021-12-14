use clap::{crate_name, crate_version, App, AppSettings, SubCommand};
use env_logger::Env;
use log::{error, info};

use talk::link::rendezvous::{Server, ServerSettings};

#[tokio::main]
async fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about("A rendezvous server where replicas register!")
        .args_from_usage("-v... 'Sets the level of verbosity'")
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs a single rendezvous server")
                .args_from_usage("--port=[INT] 'The port in which to run")
                .args_from_usage("--size=[INT] 'The number of members in the system")
                .args_from_usage("--fast_brokers=[INT] 'The number of fast brokers in the system")
                .args_from_usage("--full_brokers=[INT] 'The number of full brokers in the system")
                .args_from_usage("--num_clients=[INT] 'The number of clients in the system"),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    let log_level = match matches.occurrences_of("v") {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        _ => "trace",
    };
    let mut logger = env_logger::Builder::from_env(Env::default().default_filter_or(log_level));
    #[cfg(feature = "benchmark")]
    logger.format_timestamp_millis();
    logger.init();

    match matches.subcommand() {
        ("run", Some(subm)) => {
            let port = subm
                .value_of("port")
                .unwrap_or("9000")
                .parse::<u16>()
                .unwrap();
            let shard_size = subm.value_of("size").unwrap().parse::<usize>().unwrap();
            let fast_brokers = subm
                .value_of("fast_brokers")
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let full_brokers = subm
                .value_of("full_brokers")
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let clients = subm
                .value_of("num_clients")
                .unwrap()
                .parse::<usize>()
                .unwrap();

            let address = ("0.0.0.0", port);

            info!("Rendezvous server starting...");

            let server = Server::new(
                address,
                ServerSettings {
                    shard_sizes: vec![
                        shard_size,
                        fast_brokers + full_brokers + clients,
                        full_brokers * 100,
                        full_brokers * 100,
                        full_brokers * 100,
                    ],
                },
            )
            .await;

            match server {
                Ok(_server) => {
                    info!("Rendezvous server online!");
                    std::future::pending::<()>().await;
                }
                Err(e) => error!("Failed to deploy rendezvous server: {:?}", e),
            }
        }
        _ => unreachable!(),
    }
}
