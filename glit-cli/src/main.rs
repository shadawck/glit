pub mod exporter;
pub mod global_option_handler;
pub mod org_command_handler;
pub mod printer;
pub mod repository_command_handler;
pub mod user_command_handler;
pub mod utils;
use std::fs::{self, File};

use ahash::HashMap;
use clap::{crate_version, Arg, Command};
use exporter::Exporter;
use glit_core::{
    org::{Org, OrgFactory},
    repo::{Repository, RepositoryFactory},
    types::RepoName,
    user::{User, UserFactory},
    Logger,
};
use global_option_handler::GlobalOptionHandler;
use org_command_handler::OrgCommandHandler;
use printer::Printer;
use repository_command_handler::RepoCommandHandler;
use reqwest::ClientBuilder;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
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

    let client = ClientBuilder::new().build().unwrap();
    let global_config = GlobalOptionHandler::config(&matches);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    match matches.subcommand() {
        Some(("repo", sub_match)) => {
            let repository_config = RepoCommandHandler::config(sub_match);
            let repository: Repository = RepositoryFactory::with_config(repository_config).create();

            let repo_extraction = repository.extract_log();

            let printer = Printer::new(global_config.clone());
            printer.print_repo(&repo_extraction);

            let exporter = Exporter::new(global_config);
            exporter.export_repo(&repo_extraction)
        }
        Some(("user", sub_match)) => {
            let user_config = UserCommandHandler::config(sub_match);
            let user: User = UserFactory::with_config(user_config)
                .build_with_client(&client)
                .await;

            let user_with_log = Logger::log_for(user, &client).await;

            //let printer = Printer::new(global_config.clone());
            //printer.print_user(&user_with_log);

            let exporter = Exporter::new(global_config.clone());
            exporter.export_user(&user_with_log)
        }
        Some(("org", sub_match)) => {
            let org_config = OrgCommandHandler::config(sub_match);
            let org: Org = OrgFactory::with_config(org_config)
                .build_with_client(&client)
                .await;

            let org_with_log_file = Logger::log_for(org, &client).await;

            //let printer = Printer::new(global_config.clone());
            //printer.print_org(&org_with_log_file);

            let exporter = Exporter::new(global_config.clone());
            exporter.export_org(&org_with_log_file);

            let mut data = fs::read_to_string("eleme.json").unwrap();
            data.pop();

            //let format = format!("{{\"repositories_data\" : {{{}}} }}", data);
            let format = format!("{{ {} }}", data);
            println!("{}", format);

            let data: HashMap<RepoName, Repository> = serde_json::from_str(&format).unwrap();

            let file = File::create("eleme_json.json").unwrap();

            let json: String = serde_json::to_string(&data).unwrap();

            fs::write("eleme_json.json", json).unwrap();
        }
        _ => {}
    }
}
