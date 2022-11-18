pub mod exporter;
pub mod global_option_handler;
pub mod org_command_handler;
pub mod printer;
pub mod repository_command_handler;
pub mod user_command_handler;
pub mod utils;

use ahash::HashMap;
use std::time::{Duration, Instant};

use glit_core::{
    org::{Org, OrgCommitData, OrgFactory},
    repo::{Repository, RepositoryCommitData, RepositoryFactory},
    user::{User, UserCommitData, UserFactory},
    CommittedDataExtraction,
};

use exporter::Exporter;
use global_option_handler::GlobalOptionHandler;
use org_command_handler::OrgCommandHandler;
use printer::Printer;
use repository_command_handler::RepoCommandHandler;
use reqwest::ClientBuilder;
use user_command_handler::UserCommandHandler;

use clap::{crate_version, Arg, Command};

#[tokio::main]
async fn main() {
    let matches = Command::new("glit")
        .version(crate_version!())
        .author("Shadawck <shadawck@protonmail.com>")
        .about("Osint tool - Extract mail from repository/user/organization by crawling commit metadata.")

        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Add information on commit hash, username ...")
                .num_args(0),
        )
        .arg(
            Arg::new("output")
                .value_name("PATH")
                .short('o')
                .long("output")
                .help("export data to json")
                .num_args(1),
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

    // Use governor to limit client query
    let client = ClientBuilder::new().build().unwrap();
    let global_config = GlobalOptionHandler::config(&matches);

    match matches.subcommand() {
        Some(("repo", sub_match)) => {
            let repository_config = RepoCommandHandler::config(sub_match);
            let repository: Repository = RepositoryFactory::with_config(repository_config).create();

            let repo_extraction: HashMap<String, RepositoryCommitData> =
                repository.committed_data();

            let printer = Printer::new(global_config.clone());
            printer.print_repo(&repo_extraction);

            let exporter = Exporter::new(global_config);
            exporter.export_repo(&repo_extraction)
        }
        Some(("user", sub_match)) => {
            let user_config = UserCommandHandler::config(sub_match);
            let user: User = UserFactory::with_config(user_config).create(&client).await;
            let user_extraction: HashMap<String, UserCommitData> = user.committed_data();

            let printer = Printer::new(global_config.clone());
            printer.print_user(&user_extraction);

            let exporter = Exporter::new(global_config.clone());
            exporter.export_user(&user_extraction)
        }
        Some(("org", sub_match)) => {
            let org_config = OrgCommandHandler::config(sub_match);
            //let org: Org = OrgFactory::with_config(org_config).create(&client).await;

            let org_extraction = OrgFactory::with_config(org_config, &client)
                .await
                .create_producer(&client)
                .await;

            //let org_extraction: HashMap<String, OrgCommitData> = org.committed_data();

            //let printer = Printer::new(global_config.clone());
            //printer.print_org(&org_extraction);
            //
            //let exporter = Exporter::new(global_config.clone());
            //exporter.export_org(&org_extraction)
        }
        _ => {}
    }
}
