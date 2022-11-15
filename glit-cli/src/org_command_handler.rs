use clap::ArgMatches;
use glit_core::config::OrgConfig;
use reqwest::Url;

use crate::utils::fix_input_url;

pub struct OrgCommandHandler {}

impl OrgCommandHandler {
    pub fn config(subcommand_match: &ArgMatches) -> OrgConfig {
        let org_url = subcommand_match
            .get_one::<String>("org_url")
            .unwrap()
            .as_str();

        let all_branches = subcommand_match
            .get_one::<bool>("all_branches")
            .unwrap()
            .to_owned();

        let org_url = fix_input_url(org_url);

        OrgConfig {
            url: Url::parse(&org_url).unwrap(),
            all_branches,
        }
    }
}
