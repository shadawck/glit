use clap::ArgMatches;
use glit_core::config::RepositoryConfig;
use reqwest::Url;

use crate::utils::fix_input_url;

pub struct RepoCommandHandler {}

impl RepoCommandHandler {
    pub fn config(subcommand_match: &ArgMatches) -> RepositoryConfig {
        let input_repository_url = subcommand_match
            .get_one::<String>("repo_url")
            .unwrap()
            .as_str();

        let branchs = subcommand_match
            .get_one::<String>("branch")
            .unwrap_or(&String::new())
            .to_owned()
            .split_terminator(",")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let all_branches = subcommand_match
            .get_one::<bool>("all_branches")
            .unwrap()
            .to_owned();

        let repository_url = fix_input_url(input_repository_url);

        // Fail fast -> Check repository and branch existence

        RepositoryConfig {
            url: Url::parse(&repository_url).unwrap(),
            branchs,
            all_branches,
        }
    }
}
