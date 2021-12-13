use carbon::external::{FastBroker, FullBroker};

use clap::{crate_name, crate_version, App, AppSettings, SubCommand};

use env_logger::Env;

use log::{error, info};

#[tokio::main]
async fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about("A research implementation of Carbon!")
        .args_from_usage("-v... 'Sets the level of verbosity'")
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs a single broker")
                .args_from_usage(
                    "--rendezvous=<STRING> 'The ip address of the server to rendezvous at'",
                )
                .args_from_usage(
                    "--rate=<INT> 'The maximum throughput rate at which to send transactions'",
                )
                .args_from_usage("--full=<BOOL> 'Whether this broker is a full broker or not'")
                .args_from_usage("--parameters=[FILE] 'The file containing the broker parameters'"),
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

    info!("Starting broker");

    match matches.subcommand() {
        ("run", Some(subm)) => {
            let rendezvous = subm.value_of("rendezvous").unwrap().to_string();
            let rate = subm.value_of("rate").unwrap().parse::<usize>().unwrap();
            let full = subm.value_of("full").unwrap().to_string() == String::from("true");
            let parameters_file = subm.value_of("parameters");

            if full {
                info!("Creating full broker!");
                match FullBroker::new(rendezvous, parameters_file, rate).await {
                    Ok(_broker) => {
                        info!("Full broker done!");
                        std::future::pending::<()>().await;
                    }
                    Err(e) => error!("{}", e),
                }
            } else {
                info!("Creating fast broker!");
                match FastBroker::new(rendezvous, parameters_file, rate).await {
                    Ok(_broker) => {
                        info!("Fast broker done!");
                        std::future::pending::<()>().await;
                    }
                    Err(e) => error!("{}", e),
                }
            }
        }
        _ => unreachable!(),
    }
}
