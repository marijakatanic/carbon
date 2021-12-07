use clap::{crate_name, crate_version, App, AppSettings, SubCommand};
use env_logger::Env;
use log::{error, info};
use tokio::net::ToSocketAddrs;

use doomstack::{here, Doom, ResultExt, Top};

#[derive(Doom)]
pub(crate) enum ReplicaError {
    #[doom(description("Fail"))]
    Fail,
}

#[tokio::main]
async fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about("A research implementation of Carbon!")
        .args_from_usage("-v... 'Sets the level of verbosity'")
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs a single node")
                .args_from_usage(
                    "--rendezvous=<STRING> 'The ip address of the server to rendezvous at'",
                )
                .args_from_usage("--discovery=<STRING> 'The ip address of the discovery server'")
                .args_from_usage("--parameters=[FILE] 'The file containing the node parameters'"),
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
            let rendezvous = subm.value_of("rendezvous").unwrap();
            let discovery = subm.value_of("discovery").unwrap();

            match Replica::new(rendezvous, discovery).await {
                Ok(_) => info!("Replica terminating successfully"),
                Err(e) => error!("{}", e),
            }
        }
        _ => unreachable!(),
    }
}

struct Replica {}

impl Replica {
    pub async fn new<A: ToSocketAddrs>(
        rendezvous: A,
        discovery: A,
    ) -> Result<(), Top<ReplicaError>> {
        Ok(())
    }
}
