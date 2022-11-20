use std::{sync::mpsc, thread};

//use std::collections::HashMap;
use ahash::{HashMap, RandomState};

use colored::Colorize;
use dashmap::DashMap;
use futures::future::join_all;
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::{
    config::{RepositoryConfig, UserConfig},
    repo::{Committers, Repository, RepositoryFactory},
    types::RepoName,
};

use crate::org::Factory;

const NUMBER_OF_REPO_PER_PAGE: u32 = 30;

#[derive(Serialize)]
pub struct User {
    pub name: String,
    #[serde(skip)]
    pub url: Url,
    pub repo_count: u32,
    #[serde(skip)]
    pub pages_urls: Vec<Url>,
    #[serde(skip)]
    pub all_branches: bool,
    pub repositories_data: DashMap<RepoName, Repository, RandomState>,
}

pub struct UserFactory {
    url: Url,
    name: String,
    page_url: Url,
    all_branches: bool,
}

impl UserFactory {
    pub fn with_config(user_config: UserConfig) -> Self {
        // CLI param
        let url = user_config.url;
        let all_branches: bool = user_config.all_branches;

        // Craft other param
        let mut path_segment = url.path_segments().unwrap();
        let name = path_segment.next().unwrap().to_string();

        let page_url = format!("{}?tab=repositories&type=source", url);
        let page_url = Url::parse(&page_url).unwrap();

        UserFactory {
            url,
            name,
            page_url,
            all_branches,
        }
    }

    pub async fn build_with_client(self, client: &Client) -> User {
        let repo_count = Self::_repositories_count(client, self.page_url.clone()).await;
        let pages_count = Self::_pages_count(repo_count);
        let pages_urls = Self::_build_repo_links(self.page_url, pages_count);

        User {
            name: self.name,
            url: self.url,
            repo_count,
            pages_urls,
            all_branches: self.all_branches,
            repositories_data: DashMap::<_, _, RandomState>::with_capacity_and_hasher(
                repo_count as usize,
                RandomState::new(),
            ),
        }
    }
}
