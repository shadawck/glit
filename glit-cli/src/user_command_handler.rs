use clap::ArgMatches;
use glit_core::config::UserConfig;
use reqwest::Url;

pub struct UserCommandHandler {}

impl UserCommandHandler {
    pub fn config(subcommand_match: &ArgMatches) -> UserConfig {
        let user_url = subcommand_match
            .get_one::<String>("user_url")
            .unwrap()
            .as_str();

        let verbose = subcommand_match
            .get_one::<bool>("verbose")
            .unwrap_or(&false)
            .to_owned();

        UserConfig {
            url: Url::parse(user_url).unwrap(),
            verbose,
        }
    }
}
