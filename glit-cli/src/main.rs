pub mod exporter;
pub mod global_option_handler;
pub mod org_command_handler;
pub mod printer;
pub mod repository_command_handler;
pub mod user_command_handler;

use std::collections::HashMap;

use glit_core::{
    org::{Org, OrgCommitData, OrgFactory},
    repo::{Repository, RepositoryCommitData, RepositoryFactory},
    user::{User, UserCommitData, UserFactory},
    CommittedDataExtraction,
};

use global_option_handler::GlobalOptionHandler;
use org_command_handler::OrgCommandHandler;
use printer::Printer;
use repository_command_handler::RepoCommandHandler;
use reqwest::{Client, ClientBuilder};
use user_command_handler::UserCommandHandler;

use clap::{crate_version, Arg, Command};

#[tokio::main]
async fn main() {
    let matches = Command::new("glit")
        .version(crate_version!())
        .author("Shadawck <shadawck@protonmail.com>")
        .about("Osint tool - Extract mail from git.")
        //.arg(
        //    Arg::new("proxy")
        //        .short('x')
        //        .long("proxy")
        //        .help("Pass through a proxy to minimize the risk to be blocked by github.")
        //        .num_args(1),
        //)
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Add information on commit hash, username ...")
                .num_args(0),
        )
        .subcommand(
            Command::new("repo")
                .about("Extract emails from repository by crawling commit metadata.")
                .arg(
                    Arg::new("repo_url")
                        .value_name("URL")
                        .short('u')
                        .long("url")
                        .help("Github url of a repository"),
                )
                .arg(
                    Arg::new("branch")
                        .short('b')
                        .long("branch")
                        .help("Select a specific branch (default : master | main)")
                        .num_args(1..),
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
                        .help("Github url of an organisation."),
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
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("Add information on commit hash, username ...")
                        .num_args(0),
                ),
        )
        .get_matches();

    let client = ClientBuilder::new().build().unwrap();
    let global_config = GlobalOptionHandler::config(&matches);

    match matches.subcommand() {
        Some(("repo", sub_match)) => {
            let repository_config = RepoCommandHandler::config(sub_match);
            let repository: Repository =
                RepositoryFactory::with_config(repository_config.clone()).create();
            let repo_extraction: HashMap<String, RepositoryCommitData> =
                repository.clone().committed_data();

            let printer = Printer::new(repository_config, global_config, repository);
            printer.print(repo_extraction);
        }
        Some(("user", sub_match)) => {
            let user_config = UserCommandHandler::config(sub_match);
            let user: User = UserFactory::with_config(user_config).create(&client); //
            let user_extraction: UserCommitData = user.committed_data();
        }
        Some(("org", sub_match)) => {
            let org_config = OrgCommandHandler::config(sub_match);
            let org: Org = OrgFactory::with_config(org_config).create(&client);
            let org_extraction: OrgCommitData = org.committed_data();
        }
        _ => {}
    }
}
