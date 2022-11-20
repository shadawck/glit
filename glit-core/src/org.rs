use std::{borrow::BorrowMut, sync::Arc, time::Instant};

use ahash::RandomState;
use async_trait::async_trait;
use crossbeam::channel::bounded;
use dashmap::DashMap;
use futures::future::join_all;
use rayon::ThreadPoolBuilder;
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Serialize;

const NUMBER_OF_REPO_PER_PAGE: u32 = 30;

use crate::{
    config::{OrgConfig, RepositoryConfig},
    repo::{Repository, RepositoryFactory},
    types::RepoName,
    user::{User, UserFactory},
};

#[derive(Debug, Clone, Serialize)]
pub struct Org {
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

pub struct OrgFactory {
    url: Url,
    name: String,
    page_url: Url,
    all_branches: bool,
}

// Function common to Userfactory and OrgFactory
#[async_trait]
pub trait Factory {
    async fn _repositories_count(client: &Client, url: Url) -> u32;

    fn _pages_count(repo_count: u32) -> u32 {
        let modulo = repo_count % NUMBER_OF_REPO_PER_PAGE;
        if modulo.eq(&0) {
            repo_count / NUMBER_OF_REPO_PER_PAGE
        } else {
            ((repo_count - modulo) / NUMBER_OF_REPO_PER_PAGE) + 1
        }
    }

    fn _build_repo_links(page_url: Url, pages_count: u32) -> Vec<Url> {
        let mut pages_urls = Vec::new();
        for i in 1..pages_count + 1 {
            let url = format!("{}&page={}", page_url, i);
            pages_urls.push(Url::parse(&url).unwrap());
        }

        pages_urls
    }
}

#[async_trait]
impl Factory for OrgFactory {
    async fn _repositories_count(client: &Client, url: Url) -> u32 {
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
            .parse::<u32>()
            .unwrap()
    }
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
        let pages_urls = Self::_build_repo_links(self.page_url, pages_count);

        Org {
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

#[async_trait]
impl Factory for UserFactory {
    async fn _repositories_count(client: &Client, url: Url) -> u32 {
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
            .parse::<u32>()
            .unwrap()
    }
}

impl User {}

#[async_trait]
pub trait ExtractLog {
    async fn common_log_feature(
        &self,
        client: &Client,
        selector: Selector,
    ) -> DashMap<RepoName, Repository, ahash::RandomState> {
        let start_a = Instant::now();

        let repo_count = self.get_repo_count();
        let all_branches = self.get_all_branches();
        let pages_urls = self.get_pages_url();
        let url = self.get_url();

        let (tx_url, rx_url) = bounded(repo_count as usize);
        let mut tokio_handles = Vec::new();
        for page in pages_urls {
            let client = client.clone();
            let url = url.clone();
            let tx_url = tx_url.clone();
            let selector = selector.clone();

            let handle = tokio::spawn(async move {
                let resp = client.get(page).send().await.unwrap();
                let text = resp.text().await.unwrap();
                let parser = Html::parse_document(&text);

                parser
                    .select(&selector)
                    .map(|link| {
                        let endpoint_url = link.value().attr("href").unwrap().to_string();
                        let repo_name = endpoint_url.split('/').last().unwrap();
                        let repo_url = format!("{}{}/", url, repo_name);
                        println!("Send url {}", repo_url.to_string());
                        tx_url.send(Url::parse(&repo_url).unwrap()).unwrap();
                        drop(&tx_url)
                    })
                    .for_each(drop);
            });

            tokio_handles.push(handle);
        }
        drop(tx_url);

        //let queue = Arc::new(ArrayQueue::new(self.repo_count.try_into().unwrap()));
        let mut queue_handles = Vec::new();
        let (tx, rx) = bounded(repo_count.try_into().unwrap());

        for i in 0..repo_count {
            //let queue = queue.clone();
            let tx = tx.clone();
            let rx_url = rx_url.clone();

            let handle = rayon::spawn(move || {
                let clonable_url = rx_url.recv().unwrap();

                let repo_config = RepositoryConfig {
                    url: clonable_url.clone(),
                    all_branches,
                };

                let repo = RepositoryFactory::with_config(repo_config).create();

                tx.send(repo).unwrap();
                drop(tx);

                println!("Repo Cloned and pushed to queue");
            });

            queue_handles.push(handle)
        }
        drop(tx);
        drop(rx_url);

        let pool = ThreadPoolBuilder::new().num_threads(8).build().unwrap();
        let hasher = RandomState::new();
        let mut dash: Arc<DashMap<RepoName, Repository, RandomState>> = Arc::new(
            DashMap::with_capacity_and_hasher(repo_count as usize, hasher),
        );

        let dash_result = pool.scope(move |scope| {
            for _ in 0..repo_count {
                let dash = dash.clone();
                let rx = rx.clone();

                scope.spawn(move |_| {
                    // let loc = queue.pop().unwrap();
                    let repo = rx.recv().unwrap();
                    drop(rx);
                    let data = repo.clone().extract_log();

                    let repo_name_key = RepoName(repo.name);
                    dash.insert(repo_name_key, data);
                })
            }
            dash
        });
        drop(pool);

        join_all(tokio_handles).await;

        println!(
            " Fetching and Cloning handled in {:?} for {}",
            start_a.elapsed(),
            repo_count
        );

        Arc::try_unwrap(dash_result).unwrap()
    }

    async fn extract_log(mut self, client: &Client) -> Self;

    // Getter
    fn get_repo_count(&self) -> u32;
    fn get_all_branches(&self) -> bool;
    fn get_url(&self) -> Url;
    fn get_pages_url(&self) -> Vec<Url>;
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

    fn get_repo_count(&self) -> u32 {
        self.repo_count
    }

    fn get_pages_url(&self) -> Vec<Url> {
        self.pages_urls.clone()
    }
}

#[async_trait]
impl ExtractLog for User {
    async fn extract_log(mut self, client: &Client) -> Self {
        let user_selector =
            Selector::parse(r#"turbo-frame > div > div > ul > li > div > div > h3 > a"#).unwrap();

        self.repositories_data = Self::common_log_feature(&self, client, user_selector).await;
        self
    }

    fn get_repo_count(&self) -> u32 {
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
}

pub struct Logger;

impl Logger {
    pub async fn log_for<T: ExtractLog>(t: T, client: &Client) -> T {
        t.extract_log(client).await
    }
}
