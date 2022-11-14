use clap::ArgMatches;
use glit_core::config::UserConfig;
use reqwest::Url;

use crate::utils::fix_input_url;

pub struct UserCommandHandler {}

impl UserCommandHandler {
    pub fn config(subcommand_match: &ArgMatches) -> UserConfig {
        let user_url = subcommand_match
            .get_one::<String>("user_url")
            .unwrap()
            .as_str();

        let all_branches = subcommand_match
            .get_one::<bool>("all_branches")
            .unwrap()
            .to_owned();

        let user_url = fix_input_url(user_url);

        UserConfig {
            url: Url::parse(&user_url).unwrap(),
            all_branches,
        }
    }
}
