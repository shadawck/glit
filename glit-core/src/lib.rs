use crate::{config::RepositoryConfig, repo::RepositoryFactory};
use ahash::RandomState;
use async_trait::async_trait;
use crossbeam_channel::bounded;
use dashmap::DashMap;
//use futures_util::future::join_all;
//use rayon::ThreadPoolBuilder;
use repo::Repository;
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};
use tracing::error;
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
        let mut pages_urls = Vec::with_capacity(repo_count);
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
        //let url = self.get_url().clone();

        //let (tx_url, rx_url) = bounded(repo_count);
        //let mut tokio_handles = Vec::with_capacity(pages_urls.len());

        let mut reqwest_urls = Vec::new();
        for page in pages_urls {
            //    let client = client.clone();
            //    let url = url.clone();
            //    let tx_url = tx_url.clone();
            //    let selector = selector.clone();
            //
            //    //let handle = tokio::spawn(async move {
            let resp = client.get(page).send().await.unwrap();
            let text = resp.text().await.unwrap();
            let parser = Html::parse_document(&text);

            parser
                .select(&selector)
                .map(|link| {
                    let url = self.get_url().clone();
                    let endpoint_url = link.clone().value().attr("href").unwrap().to_string();
                    let repo_name = endpoint_url.split('/').last().unwrap();

                    let repo_url = format!("{}{}/", url.clone(), repo_name);

                    let url_parsed = Url::parse(&repo_url).unwrap();

                    reqwest_urls.push(url_parsed);

                    //let sending = tx_url.send(Url::parse(&repo_url).unwrap());
                    //match sending {
                    //    Ok(_) => tracing::debug!("Send url {}", repo_url),
                    //    Err(e) => {
                    //        error!("Failed to send {} with : [{:?}]", repo_url, e)
                    //    }
                    //}
                })
                .for_each(drop);

            // });
            // tokio_handles.push(handle);
        }
        //drop(tx_url);
        //
        ////let mut queue_handles = Vec::with_capacity(repo_count);
        //let (tx, rx) = bounded(repo_count);

        let mut repos = Vec::new();
        for _ in 0..repo_count {
            let reqwest_urls = reqwest_urls.clone();
            //    let tx = tx.clone();
            //    let rx_url = rx_url.clone();
            //
            //    //let handle = tokio::spawn(async move {

            for clonable_url in reqwest_urls {
                //let clonable_url = rx_url.recv().unwrap();

                let repo_config = RepositoryConfig::new(clonable_url, all_branches.clone());

                let repo = RepositoryFactory::with_config(repo_config).create();
                repos.push(repo);

                // tx.send(repo).unwrap();
                // drop(tx);
                // //})
                // //.await;
                // //queue_handles.push(handle)
            }
        }
        //drop(tx);
        //drop(rx_url);
        //
        //let num_threads = rayon::max_num_threads();
        //
        //tracing::info!("Number of threads : {}", num_threads);
        //
        //let pool = ThreadPoolBuilder::new()
        //    .num_threads(num_threads - 2)
        //    .build()
        //    .unwrap();
        //
        //let dash: Arc<DashMap<RepoName, Repository, RandomState>> = Arc::new(
        //    DashMap::with_capacity_and_hasher(repo_count, RandomState::new()),
        //);
        //let atomic_count = Arc::new(AtomicUsize::new(0));

        let dash_result = DashMap::with_capacity_and_hasher(repo_count, RandomState::new());
        ////pool.scope(move |scope| {
        for _ in 0..repo_count {
            //    let dash = dash.clone();
            //    let rx = rx.clone();
            //    let atomic_count = atomic_count.clone();
            //
            let repos = repos.clone();
            for repo in repos {
                //    //scope.spawn(move |_| {
                //    let repo = rx.recv().unwrap();
                //    drop(rx);
                //
                let data = repo.clone().extract_log();
                let repo_name_key = RepoName(repo.name.clone());
                dash_result.insert(repo_name_key, data);

                //atomic_count.fetch_add(1, Ordering::Relaxed);
                //println!(
                //    "Repository {} handled : {}/{}",
                //    repo.name,
                //    atomic_count.load(Ordering::Relaxed),
                //    repo_count
                //);
                //})
            }
        }
        //  dash
        //});
        //drop(pool);
        //
        ////join_all(tokio_handles).await;
        //
        //tracing::info!(
        //    "Fetching and Cloning handled in {:?} for {}",
        //    start_a.elapsed(),
        //    repo_count
        //);
        //
        //Arc::try_unwrap(dash_result).unwrap()

        dash_result
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
