use std::{path::PathBuf, str::FromStr};

use ahash::RandomState;
use async_trait::async_trait;
use dashmap::DashMap;
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Serialize;

use crate::{config::UserConfig, repo::Repository, types::RepoName, ExtractLog, Factory};

#[derive(Serialize)]
pub struct User {
    pub name: String,
    #[serde(skip)]
    pub url: Url,
    pub repo_count: usize,
    #[serde(skip)]
    pub pages_urls: Vec<Url>,
    #[serde(skip)]
    pub all_branches: bool,
    pub data_file: PathBuf, //pub repositories_data: DashMap<RepoName, Repository, RandomState>,
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
        let pages_urls = Self::_build_repo_links(self.page_url, repo_count, pages_count);
        let data_file = PathBuf::from_str(format!("{}.json", self.name).as_str()).unwrap();

        User {
            name: self.name,
            url: self.url,
            repo_count,
            pages_urls,
            all_branches: self.all_branches,
            data_file,
            //repositories_data: DashMap::<_, _, RandomState>::with_capacity_and_hasher(
            //    repo_count,
            //    RandomState::new(),
            //),
        }
    }
}

#[async_trait]
impl Factory for UserFactory {
    async fn _repositories_count(client: &Client, url: Url) -> usize {
        let resp = client.get(url).send().await.unwrap();
        let text = resp.text().await.unwrap();

        let parser = Html::parse_document(&text);
        let selector_repositories_count =
            Selector::parse(r#"turbo-frame > div > div > div > div > strong"#).unwrap();

        let repository_count_str = parser
            .select(&selector_repositories_count)
            .next()
            .unwrap()
            .inner_html();

        repository_count_str
            .trim()
            .replace(',', "")
            .parse::<usize>()
            .unwrap()
    }
}

#[async_trait]
impl ExtractLog for User {
    async fn extract_log(mut self, client: &Client) -> Self {
        let user_selector =
            Selector::parse(r#"turbo-frame > div > div > ul > li > div > div > h3 > a"#).unwrap();

        self.data_file = Self::common_log_feature(&self, client, user_selector).await;
        self
    }

    fn get_repo_count(&self) -> usize {
        self.repo_count
    }

    fn get_all_branches(&self) -> bool {
        self.all_branches
    }

    fn get_url(&self) -> Url {
        self.url.clone()
    }

    fn get_pages_url(&self) -> Vec<Url> {
        self.pages_urls.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}
