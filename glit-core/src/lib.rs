use crate::{config::RepositoryConfig, repo::RepositoryFactory};
use ahash::RandomState;
use async_trait::async_trait;
use crossbeam::channel::bounded;
use dashmap::DashMap;
use futures::future::join_all;
use rayon::ThreadPoolBuilder;
use repo::Repository;
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::{sync::Arc, time::Instant};
use tracing::{error, info};
use types::RepoName;

pub mod config;
pub mod log;
pub mod org;
pub mod repo;
pub mod types;
pub mod user;

const NUMBER_OF_REPO_PER_PAGE: usize = 30;

#[async_trait]
pub trait Factory {
    async fn _repositories_count(client: &Client, url: Url) -> usize;

    fn _pages_count(repo_count: usize) -> usize {
        let modulo = repo_count % NUMBER_OF_REPO_PER_PAGE;
        if modulo.eq(&0) {
            repo_count / NUMBER_OF_REPO_PER_PAGE
        } else {
            ((repo_count - modulo) / NUMBER_OF_REPO_PER_PAGE) + 1
        }
    }

    fn _build_repo_links(page_url: Url, repo_count: usize, pages_count: usize) -> Vec<Url> {
        let mut pages_urls = Vec::with_capacity(repo_count as usize);
        for i in 1..pages_count + 1 {
            let url = format!("{}&page={}", page_url, i);
            pages_urls.push(Url::parse(&url).unwrap());
        }

        pages_urls
    }
}

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
        let mut tokio_handles = Vec::with_capacity(pages_urls.len());
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
                        let sending = tx_url.send(Url::parse(&repo_url).unwrap());
                        match sending {
                            Ok(_) => info!("Send url {}", repo_url),
                            Err(e) => {
                                error!("Failed to send {} with : [{:?}]", repo_url, e)
                            }
                        }
                    })
                    .for_each(drop);
            });

            tokio_handles.push(handle);
        }
        drop(tx_url);

        //let queue = Arc::new(ArrayQueue::new(self.repo_count.try_into().unwrap()));
        let mut queue_handles = Vec::with_capacity(repo_count as usize);
        let (tx, rx) = bounded(repo_count.try_into().unwrap());

        for _ in 0..repo_count {
            //let queue = queue.clone();
            let tx = tx.clone();
            let rx_url = rx_url.clone();

            let handle = rayon::spawn(move || {
                let clonable_url = rx_url.recv().unwrap();

                let repo_config = RepositoryConfig {
                    url: clonable_url,
                    all_branches,
                };

                let repo = RepositoryFactory::with_config(repo_config).create();

                tx.send(repo).unwrap();
                drop(tx);
            });

            queue_handles.push(handle)
        }
        drop(tx);
        drop(rx_url);

        let pool = ThreadPoolBuilder::new().num_threads(8).build().unwrap();
        let hasher = RandomState::new();
        let dash: Arc<DashMap<RepoName, Repository, RandomState>> = Arc::new(
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

        info!(
            "Fetching and Cloning handled in {:?} for {}",
            start_a.elapsed(),
            repo_count
        );

        Arc::try_unwrap(dash_result).unwrap()
    }

    async fn extract_log(mut self, client: &Client) -> Self;

    // Common Getter
    fn get_repo_count(&self) -> usize;
    fn get_all_branches(&self) -> bool;
    fn get_url(&self) -> Url;
    fn get_pages_url(&self) -> Vec<Url>;
}

pub struct Logger;
impl Logger {
    pub async fn log_for<T: ExtractLog>(t: T, client: &Client) -> T {
        t.extract_log(client).await
    }
}
