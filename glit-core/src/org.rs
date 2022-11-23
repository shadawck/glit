use ahash::RandomState;
use async_trait::async_trait;
use dashmap::DashMap;
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Serialize;

use crate::{config::OrgConfig, repo::Repository, types::RepoName, ExtractLog, Factory};

#[derive(Debug, Clone, Serialize)]
pub struct Org {
    pub name: String,
    #[serde(skip)]
    pub url: Url,
    pub repo_count: usize,
    #[serde(skip)]
    pub pages_urls: Vec<Url>,
    #[serde(skip)]
    pub all_branches: bool,
    pub repositories_data: DashMap<RepoName, Repository, RandomState>,
}

pub struct OrgFactory {
    url: Url,
    name: String,
    page_url: Url,
    all_branches: bool,
}

impl OrgFactory {
    pub fn with_config(org_config: OrgConfig) -> Self {
        // CLI param
        let url = org_config.url;
        let all_branches: bool = org_config.all_branches;

        // Craft other param
        let mut path_segment = url.path_segments().unwrap();
        let name = path_segment.next().unwrap().to_string();

        let host = url.host().unwrap().to_string();
        let scheme = url.scheme();

        let page_url = format!(
            "{}://{}/orgs/{}/repositories?q=&type=source",
            scheme, host, name
        );
        let page_url = Url::parse(&page_url).unwrap();

        Self {
            url,
            name,
            page_url,
            all_branches,
        }
    }

    pub async fn build_with_client(self, client: &Client) -> Org {
        let repo_count = Self::_repositories_count(client, self.page_url.clone()).await;
        let pages_count = Self::_pages_count(repo_count);
        let pages_urls = Self::_build_repo_links(self.page_url, repo_count, pages_count);

        Org {
            name: self.name,
            url: self.url,
            repo_count,
            pages_urls,
            all_branches: self.all_branches,
            repositories_data: DashMap::<_, _, RandomState>::with_capacity_and_hasher(
                repo_count,
                RandomState::new(),
            ),
        }
    }
}

#[async_trait]
impl Factory for OrgFactory {
    async fn _repositories_count(client: &Client, url: Url) -> usize {
        let resp = client.get(url).send().await.unwrap();
        let text = resp.text().await.unwrap();

        let parser = Html::parse_document(&text);
        let selector_repositories_count =
            Selector::parse(r#"main > div > div > div > div > div > div > div > strong"#).unwrap();

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
impl ExtractLog for Org {
    async fn extract_log(mut self, client: &Client) -> Self {
        let org_selector = Selector::parse(
            r#"main > div > div > div > div > div > div > ul > li > div > div > div > h3 > a"#,
        )
        .unwrap();

        self.repositories_data = Self::common_log_feature(&self, client, org_selector).await;
        self
    }

    fn get_all_branches(&self) -> bool {
        self.all_branches
    }

    fn get_url(&self) -> Url {
        self.url.clone()
    }

    fn get_repo_count(&self) -> usize {
        self.repo_count
    }

    fn get_pages_url(&self) -> Vec<Url> {
        self.pages_urls.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}
