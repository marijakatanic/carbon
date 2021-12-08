use carbon::external::FullBroker;

use clap::{crate_name, crate_version, App, AppSettings, SubCommand};

use env_logger::Env;

use log::{error};

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

    match matches.subcommand() {
        ("run", Some(subm)) => {
            let rendezvous = subm.value_of("rendezvous").unwrap().to_string();
            let full = subm.value_of("full").is_some();
            // let parameters_file = subm.value_of("parameters");

            if full {
                match FullBroker::new(rendezvous).await {
                    Ok(_broker) => std::future::pending::<()>().await,
                    Err(e) => error!("{}", e),
                }
            } else {
            }
        }
        _ => unreachable!(),
    }
}
