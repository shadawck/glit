use std::{fs::remove_dir_all, path::PathBuf, str::FromStr, sync::Arc, time::Instant};

use ahash::{HashMap, RandomState};
use crossbeam::channel::bounded;
use dashmap::DashMap;
use futures::future::join_all;
use git2::build::RepoBuilder;
use rand::distributions::{Alphanumeric, DistString};
use rayon::{
    prelude::{IntoParallelRefIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

const NUMBER_OF_REPO_PER_PAGE: u32 = 30;

use crate::{
    config::{OrgConfig, RepositoryConfig},
    repo::{Repository, RepositoryCommitData, RepositoryFactory},
    types::Branch,
};
use crate::{log::Log, CommittedDataExtraction};
pub struct Org {
    name: String,
    url: Url,
    repositories: Vec<Repository>,
}

pub struct OrgFactory {
    url: Url,
    page_url: Url,
    all_branches: bool,
    repo_count: u32,
    pages_count: u32,
}

impl OrgFactory {
    pub async fn with_config(org_config: OrgConfig, client: &Client) -> Self {
        let url = org_config.url;
        let all_branches: bool = org_config.all_branches;

        let mut path_segment = url.path_segments().unwrap();
        let org_name = path_segment.next().unwrap().to_string();

        let host = url.host().unwrap().to_string();
        let scheme = url.scheme();

        let page_url = format!(
            "{}://{}/orgs/{}/repositories?q=&type=source",
            scheme, host, org_name
        );
        let page_url = Url::parse(&page_url).unwrap();
        let repo_count = Self::repositories_count(client, page_url.clone()).await;
        let pages_count = Self::pages_count(repo_count);

        OrgFactory {
            url,
            page_url,
            all_branches,
            repo_count,
            pages_count,
        }
    }

    pub async fn repositories_count(client: &Client, url: Url) -> u32 {
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

    fn pages_count(repo_count: u32) -> u32 {
        let modulo = repo_count % NUMBER_OF_REPO_PER_PAGE;
        if modulo.eq(&0) {
            repo_count / NUMBER_OF_REPO_PER_PAGE
        } else {
            ((repo_count - modulo) / NUMBER_OF_REPO_PER_PAGE) + 1
        }
    }

    async fn build_url(page_url: Url, client: &Client) -> Vec<Url> {
        let repo_count = Self::repositories_count(client, page_url.clone()).await;
        let pages_count = Self::pages_count(repo_count);

        let mut pages_urls = Vec::new();
        for i in 1..pages_count + 1 {
            let url = format!("{}&page={}", page_url, i);
            pages_urls.push(Url::parse(&url).unwrap());
        }

        pages_urls
    }

    pub async fn create_producer(
        self,
        client: &Client,
    ) -> DashMap<String, OrgCommitData, RandomState> {
        let start_a = Instant::now();
        let url = self.url;
        let page_url = self.page_url;
        let pages_urls = Self::build_url(page_url, client).await;

        let (tx_url, rx_url) = bounded(self.repo_count as usize);

        let mut tokio_handles = Vec::new();
        for page in pages_urls {
            let client = client.clone();
            let url = url.clone();
            let tx_url = tx_url.clone();

            let handle = tokio::spawn(async move {
                let resp = client.get(page).send().await.unwrap();
                let text = resp.text().await.unwrap();
                let parser = Html::parse_document(&text);

                let selector_repositories_url = Selector::parse(
                    r#"main > div > div > div > div > div > div > ul > li > div > div > div > h3 > a"#,
                ).unwrap();

                parser
                    .select(&selector_repositories_url)
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
        //join_all(tokio_handles).await;

        //let queue = Arc::new(ArrayQueue::new(self.repo_count.try_into().unwrap()));
        let mut queue_handles = Vec::new();
        let (tx, rx) = bounded(self.repo_count.try_into().unwrap());

        for i in 0..self.repo_count {
            //let queue = queue.clone();
            let tx = tx.clone();
            let rx_url = rx_url.clone();
            let all_branches = self.all_branches;

            let handle = rayon::spawn(move || {
                let clonable_url = rx_url.recv().unwrap();

                let repo_config = RepositoryConfig {
                    url: clonable_url.clone(),
                    all_branches,
                };

                let repo = RepositoryFactory::with_config(repo_config).create();

                //let mut path_segments = clonable_url.path_segments().unwrap();
                //let repo_owner = path_segments.next().unwrap();
                //let repo_name = path_segments.next().unwrap().to_string();
                //
                //let hash_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
                //let clone_location =
                //    PathBuf::from_str(&format!("{}/{}_{}", "/tmp", repo_name, hash_suffix))
                //        .unwrap();
                //
                // I/O Operation
                //RepoBuilder::new()
                //    .bare(true)
                //    .clone(clonable_url.as_str(), clone_location.clone().as_path())
                //    .unwrap();
                //
                //queue.push(clone_location).unwrap();
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
        let dash = Arc::new(DashMap::with_capacity_and_hasher(
            self.repo_count as usize,
            hasher,
        ));

        let dash_result = pool.scope(move |scope| {
            for _ in 0..self.repo_count {
                let dash = dash.clone();
                let rx = rx.clone();

                scope.spawn(move |_| {
                    // let loc = queue.pop().unwrap();
                    let repo = rx.recv().unwrap();
                    drop(rx);
                    let data = repo.clone().committed_data();

                    //let log = Log::build(clone_location.clone());

                    //dash.insert(repo.name, org);
                    //remove_dir_all(clone_location).unwrap();
                })
            }
            dash
        });
        drop(pool);

        join_all(tokio_handles).await;

        println!(
            " Fetching and Cloning handled in {:?} for {}",
            start_a.elapsed(),
            self.repo_count
        );

        Arc::try_unwrap(dash_result).unwrap()
    }

    pub async fn create(self, client: &Client) -> Org {
        let url = self.url;
        let page_url = self.page_url;

        let mut path_segment = url.path_segments().unwrap();
        let name = path_segment.next().unwrap().to_string();

        let repo_count = Self::repositories_count(client, page_url.clone()).await;
        let pages_count = Self::pages_count(repo_count);

        let mut pages_urls = Vec::new();
        for i in 1..pages_count + 1 {
            let url = format!("{}&page={}", page_url, i);
            pages_urls.push(Url::parse(&url).unwrap());
        }

        // Heavy time consuming
        let repositories =
            Self::fetch_repository_list(client, url.clone(), pages_urls, self.all_branches).await;

        Org {
            name,
            url,
            repositories,
        }
    }

    // TODO: Duplicate with user.rs
    async fn fetch_repository_list(
        client: &Client,
        base_url: Url,
        pages_urls: Vec<Url>,
        all_branches: bool,
    ) -> Vec<Repository> {
        let mut tokio_handles = Vec::new();

        // Rem : The channeling is used for only one message as each task have only one message to pass.
        let channel_len = pages_urls.len();
        println!("Channel len: {}", channel_len);

        let (tx_url, rx_url) = bounded(channel_len * NUMBER_OF_REPO_PER_PAGE as usize);
        println!("Bounded created");

        for page in pages_urls {
            println!("Entering loop");
            let client = client.clone();
            let tx_url = tx_url.clone();

            let handle = tokio::spawn(async move {
                let resp = client.get(page).send().await.unwrap();
                let text = resp.text().await.unwrap();
                let parser = Html::parse_document(&text);

                let selector_repositories_url = Selector::parse(
                    r#"main > div > div > div > div > div > div > ul > li > div > div > div > h3 > a"#,
                ).unwrap();

                parser
                    .select(&selector_repositories_url)
                    .map(|link| {
                        let endpoint_url = link.value().attr("href").unwrap().to_string();
                        println!("{}", endpoint_url);
                        tx_url.send(endpoint_url).unwrap();
                        drop(&tx_url);
                    })
                    .for_each(drop);
                drop(tx_url)
            });

            tokio_handles.push(handle);
        }
        join_all(tokio_handles).await;

        drop(tx_url);

        // One options is to just recv a repository and pass it to top calling function without waiting for
        // for all the url to be fetch.
        let urls = rx_url.try_iter().collect::<Vec<String>>(); // <--- Blocking / Barrier

        println!("Pass the blocking barrier");

        // ---- Rayon impl
        urls.par_iter()
            .map(|endpoint_url| {
                let repo_name = endpoint_url.split('/').last().unwrap();
                let repo_url = format!("{}{}/", base_url, repo_name);
                let url = Url::parse(&repo_url).unwrap();

                let repo_config = RepositoryConfig { url, all_branches };

                // Github Cloning operation

                RepositoryFactory::with_config(repo_config).create()
            })
            .collect::<Vec<Repository>>()

        //rx_url.iter().collect::<Vec<Repository>>()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgCommitData {
    pub branches: HashMap<Branch, RepositoryCommitData>,
}

//type RepoName = String;
//impl CommittedDataExtraction<HashMap<RepoName, OrgCommitData>> for Org {
//    fn committed_data(self) -> HashMap<RepoName, OrgCommitData> {
//        self.repositories
//            .par_iter()
//            .map(|repo| {
//                let branches_data = repo.to_owned().committed_data();
//                let org_commit_data = OrgCommitData {
//                    branches: branches_data,
//                };
//                (repo.name.to_owned(), org_commit_data)
//            })
//            .collect::<HashMap<_, _>>()
//    }
//}
