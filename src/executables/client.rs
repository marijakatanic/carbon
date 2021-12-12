use carbon::external::Client;

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
                .about("Runs a single client")
                .args_from_usage(
                    "--rendezvous=<STRING> 'The ip address of the server to rendezvous at'",
                )
                .args_from_usage("--parameters=[FILE] 'The file containing the client parameters'"),
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

    info!("Starting client");

    match matches.subcommand() {
        ("run", Some(subm)) => {
            let rendezvous = subm.value_of("rendezvous").unwrap().to_string();
            let parameters_file = subm.value_of("parameters");

            info!("Creating client");
            match Client::new(rendezvous, parameters_file).await {
                Ok(_broker) => {
                    info!("Full client done");
                    std::future::pending::<()>().await;
                }
                Err(e) => error!("{}", e),
            }
        }
        _ => unreachable!(),
    }
}
