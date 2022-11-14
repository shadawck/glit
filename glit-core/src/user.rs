use reqwest::{Client, Url};

use crate::{
    config::UserConfig,
    repo::{Committer, Repository, RepositoryCommitData},
    CommittedDataExtraction,
};

pub struct User {
    name: String,
    url: Url,
    repository_list: Vec<Repository>, // Network action
}

pub struct UserFactory {
    url: Url,
    verbose: bool,
}

impl UserFactory {
    pub fn with_config(user_config: UserConfig) -> Self {
        let url = user_config.url;
        let verbose = user_config.verbose;

        UserFactory { url, verbose }
    }

    pub fn create(self, client: &Client) -> User {
        let url = self.url;
        let verbose = self.verbose;

        let mut path_segment = url.path_segments().unwrap();
        let name = path_segment.next().unwrap().to_string();

        User {
            name,
            url,
            repository_list: Self::fetch_repository_list(client),
        }
    }

    fn fetch_repository_list(client: &Client) -> Vec<Repository> {
        todo!()
    }
}

pub struct UserCommitData {
    committer_data: Vec<RepositoryCommitData>,
}

impl CommittedDataExtraction<UserCommitData> for User {
    fn committed_data(self) -> UserCommitData {
        UserCommitData {
            committer_data: todo!(),
        }
    }
}
