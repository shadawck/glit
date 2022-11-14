use reqwest::{Client, Url};

use crate::{
    config::OrgConfig,
    repo::{Repository, RepositoryCommitData},
    CommittedDataExtraction,
};

pub struct Org {
    name: String,
    url: Url,
    repository_list: Vec<Repository>, // Network action
}

pub struct OrgCommitData {
    committer_data: Vec<RepositoryCommitData>,
}

pub struct OrgFactory {
    url: Url,
    verbose: bool,
}

impl OrgFactory {
    pub fn with_config(org_config: OrgConfig) -> Self {
        let url = org_config.url;
        let verbose = org_config.verbose;

        OrgFactory { url, verbose }
    }

    pub fn create(self, client: &Client) -> Org {
        let url = self.url;

        let mut path_segments = url.path_segments().unwrap();
        let name = path_segments.next().unwrap().to_string();

        Org {
            name,
            url,
            repository_list: Self::fetch_repository_list(client),
        }
    }

    fn fetch_repository_list(client: &Client) -> Vec<Repository> {
        todo!()
    }
}

impl CommittedDataExtraction<OrgCommitData> for Org {
    fn committed_data(self) -> OrgCommitData {
        OrgCommitData {
            committer_data: todo!(),
        }
    }
}
