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
                .args_from_usage("--size=[INT] 'The number of members in the system"),
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

    info!("Logger test");

    match matches.subcommand() {
        ("run", Some(subm)) => {
            let port = subm
                .value_of("port")
                .unwrap_or("9000")
                .parse::<u16>()
                .unwrap();
            let shard_size = subm.value_of("size").unwrap().parse::<usize>().unwrap();

            let address = ("0.0.0.0", port);

            info!("Rendezvous server starting...");

            let server = Server::new(
                address,
                ServerSettings {
                    shard_sizes: vec![shard_size],
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
