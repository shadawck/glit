use clap::ArgMatches;
use glit_core::config::OrgConfig;
use reqwest::Url;

pub struct OrgCommandHandler {}

impl OrgCommandHandler {
    pub fn config(subcommand_match: &ArgMatches) -> OrgConfig {
        let user_url = subcommand_match
            .get_one::<String>("org_url")
            .unwrap()
            .as_str();

        let all_branches = subcommand_match
            .get_one::<bool>("all_branches")
            .unwrap()
            .to_owned();

        OrgConfig {
            url: Url::parse(user_url).unwrap(),
            all_branches,
        }
    }
}
