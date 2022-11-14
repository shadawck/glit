use reqwest::{Client, Url};

use crate::{config::OrgConfig, org::Org, repo::Repository};

pub struct OrgFactory {
    pub(crate) url: Url,
}

impl OrgFactory {
    pub fn with_config(org_config: OrgConfig) -> Self {
        let url = org_config.url;

        OrgFactory { url }
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
