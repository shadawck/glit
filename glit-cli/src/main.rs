pub mod exporter;
pub mod global_option_handler;
pub mod org_command_handler;
pub mod printer;
pub mod repository_command_handler;
pub mod user_command_handler;
pub mod utils;
use std::time::Instant;

use clap::{crate_version, Arg, Command};
use colored::Colorize;
use exporter::Exporter;
use glit_core::{
    org::{Org, OrgFactory},
    repo::RepositoryFactory,
    user::{User, UserFactory},
    Logger,
};

use global_option_handler::GlobalOptionHandler;
use log::LevelFilter;
use org_command_handler::OrgCommandHandler;
use repository_command_handler::RepoCommandHandler;
use reqwest::ClientBuilder;

use user_command_handler::UserCommandHandler;

#[tokio::main]
async fn main() {
    let matches = Command::new("glit")
        .version(crate_version!())
        .author("Shadawck <shadawck@protonmail.com>")
        .about("Osint tool - Extract mail from repository/user/organization by crawling commit metadata.")
        .arg(
            Arg::new("verbose")
                .short('v')
                .action(clap::ArgAction::Count)
                .help("Add information on commit hash, username ...")
                .global(true)
        )
        .arg(
            Arg::new("output")
                .value_name("PATH")
                .short('o')
                .long("output")
                .help("export data to json")
                .num_args(1)
                .global(true),
        ).arg(
            Arg::new("thread")
                .short('t')
                .long("thread")
                .help("Specify the number of thread")
                .num_args(1)
                .global(true),
        )

        .subcommand(
            Command::new("repo")
                .about("Extract emails from repository")
                .arg(
                    Arg::new("repo_url")
                        .value_name("URL")
                        .short('u')
                        .long("url")
                        .help("Github url of a repository"),
                )
                .arg(
                    Arg::new("all_branches")
                        .short('a')
                        .long("all-branches")
                        .help("Get all branch of the repo")
                        .num_args(0),
                ),
        )
        .subcommand(
            Command::new("org")
                .about("Extract emails from all repositories of a github organisation.")
                .arg(
                    Arg::new("org_url")
                        .value_name("URL")
                        .short('u')
                        .long("url")
                        .help("Github url of an organisation."),
                )
                .arg(
                    Arg::new("all_branches")
                        .short('a')
                        .long("all-branches")
                        .help("Get all branch of the repo")
                        .num_args(0),
                ),
        )
        .subcommand(
            Command::new("user")
                .about("Extract emails from all repositories of a user")
                .arg(
                    Arg::new("user_url")
                        .value_name("URL")
                        .short('u')
                        .long("url")
                        .help("Github url of a user"),
                )
                .arg(
                    Arg::new("all_branches")
                        .short('a')
                        .long("all-branches")
                        .help("Get all branch of the repo")
                        .num_args(0),
                ),
        )
        .get_matches();

    let client = ClientBuilder::new().build().unwrap();
    let global_config = GlobalOptionHandler::config(&matches);

    let verbose = matches.get_count("verbose").to_owned();

    let level = match verbose {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    //env_logger::builder().filter_level(level).init();
    log::info!(
        "Brought to you by {} - {}",
        "@shadawck".bright_purple(),
        "https://github.com/shadawck".bright_cyan()
    );

    match matches.subcommand() {
        Some(("repo", sub_match)) => {
            let time = Instant::now();
            let repository_config = RepoCommandHandler::config(sub_match);

            let repository = RepositoryFactory::with_config(repository_config).create();
            let repo_extraction = repository.extract_log();

            //let printer = Printer::<Repository>::new(global_config.clone());
            //printer.print_repo(&repo_extraction);

            let exporter = Exporter::new(global_config);
            exporter.export_repo(&repo_extraction);

            log::info!("Done in {:?}", time.elapsed());
        }
        Some(("user", sub_match)) => {
            let time = Instant::now();
            let user_config = UserCommandHandler::config(sub_match);
            let user: User = UserFactory::with_config(user_config)
                .build_with_client(&client)
                .await;

            let user_with_log = Logger::log_for(user, &client).await;

            //let printer = Printer::new(global_config.clone());
            //printer.print_user(&user_with_log);

            let exporter = Exporter::new(global_config.clone());
            exporter.export_user(&user_with_log);

            log::info!("Done in {:?}", time.elapsed());
        }
        Some(("org", sub_match)) => {
            let time = Instant::now();

            let org_config = OrgCommandHandler::config(sub_match);
            let org: Org = OrgFactory::with_config(org_config)
                .build_with_client(&client)
                .await;

            let org_with_log = Logger::log_for(org, &client).await;

            //let printer = Printer::new(global_config.clone());
            //printer.print_org(&org_with_log);

            let exporter = Exporter::new(global_config.clone());
            exporter.export_org(&org_with_log);

            log::info!("Done in {:?}", time.elapsed());
        }
        _ => {}
    }
}
