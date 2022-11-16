use std::{sync::mpsc, thread};

use ahash::HashMap;
//use std::collections::HashMap;
use futures::future::join_all;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

const NUMBER_OF_REPO_PER_PAGE: u32 = 30;

use crate::{
    config::{OrgConfig, RepositoryConfig},
    repo::{Repository, RepositoryCommitData, RepositoryFactory},
    CommittedDataExtraction,
};

pub struct Org {
    name: String,
    url: Url,
    repositories: Vec<Repository>,
}

pub struct OrgFactory {
    url: Url,
    page_url: Url,
    all_branches: bool,
}

impl OrgFactory {
    pub fn with_config(org_config: OrgConfig) -> Self {
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

        OrgFactory {
            url,
            page_url,
            all_branches,
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

    // TODO: Duplicate with User.rs
    fn pages_count(repo_count: u32) -> u32 {
        let modulo = repo_count % NUMBER_OF_REPO_PER_PAGE;
        if modulo.eq(&0) {
            repo_count / NUMBER_OF_REPO_PER_PAGE
        } else {
            ((repo_count - modulo) / NUMBER_OF_REPO_PER_PAGE) + 1
        }
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

        let repositories =
            Self::fetch_repository_list_rayon(client, url.clone(), pages_urls, self.all_branches)
                .await;

        Org {
            name,
            url,
            repositories,
        }
    }

    // TODO: Duplicate with user.rs
    async fn fetch_repository_list_rayon(
        client: &Client,
        base_url: Url,
        pages_urls: Vec<Url>,
        all_branches: bool,
    ) -> Vec<Repository> {
        let mut tokio_handles = Vec::new();

        let (tx, rx) = mpsc::channel();

        for page in pages_urls {
            let client = client.clone();
            let base_url = base_url.clone();
            let tx = tx.clone();

            let handle = tokio::spawn(async move {
                let client = &client.clone();

                let resp = client.get(page).send().await.unwrap();
                let text = resp.text().await.unwrap();

                let parser = Html::parse_document(&text);

                let selector_repositories_url = Selector::parse(
                    r#"main > div > div > div > div > div > div > ul > li > div > div > div > h3 > a"#,
                ).unwrap();

                parser
                    .select(&selector_repositories_url)
                    .map(|l| {
                        let endpoint_url = l.value().attr("href").unwrap().to_string();

                        let repo_name = endpoint_url.split('/').last().unwrap();

                        let repo_url = format!("{}{}/", base_url, repo_name);

                        println!("repo_url : {}", repo_url);

                        let url = Url::parse(&repo_url).unwrap();
                        println!("u: {}", url);

                        tx.send(url).unwrap();
                    })
                    .for_each(drop);
            });

            tokio_handles.push(handle);
        }
        join_all(tokio_handles).await;
        drop(tx);

        let urls = rx.iter().collect::<Vec<Url>>();

        // ---- Rayon impl - No Gain
        urls.into_par_iter()
            .map(|u| {
                let repo_config = RepositoryConfig {
                    url: u,
                    branchs: Vec::new(),
                    all_branches,
                };

                RepositoryFactory::with_config(repo_config).create()
            })
            .collect::<Vec<Repository>>()
    }

    // TODO: Duplicate with user.rs
    async fn fetch_repository_list(
        client: &Client,
        base_url: Url,
        pages_urls: Vec<Url>,
        all_branches: bool,
    ) -> Vec<Repository> {
        let mut tokio_handles = Vec::new();

        let (tx, rx) = mpsc::channel();

        for page in pages_urls {
            let client = client.clone();
            let base_url = base_url.clone();
            let tx = tx.clone();

            let handle = tokio::spawn(async move {
                let client = &client.clone();

                let resp = client.get(page).send().await.unwrap();
                let text = resp.text().await.unwrap();

                let parser = Html::parse_document(&text);

                let selector_repositories_url = Selector::parse(
                    r#"main > div > div > div > div > div > div > ul > li > div > div > div > h3 > a"#,
                ).unwrap();

                parser
                    .select(&selector_repositories_url)
                    .map(|l| {
                        let endpoint_url = l.value().attr("href").unwrap().to_string();

                        let repo_name = endpoint_url.split('/').last().unwrap();

                        let repo_url = format!("{}{}/", base_url, repo_name);

                        println!("repo_url : {}", repo_url);

                        let url = Url::parse(&repo_url).unwrap();
                        println!("u: {}", url);

                        tx.send(url).unwrap();
                    })
                    .for_each(drop);
            });

            tokio_handles.push(handle);
        }
        join_all(tokio_handles).await;
        drop(tx);

        let urls = rx.iter().collect::<Vec<Url>>();

        let mut thread_handle = Vec::new();
        let (tx, rx) = mpsc::channel();
        for u in urls {
            let tx = mpsc::Sender::clone(&tx);
            let handle = thread::spawn(move || {
                let repo_config = RepositoryConfig {
                    url: u,
                    branchs: Vec::new(),
                    all_branches,
                };

                let repo = RepositoryFactory::with_config(repo_config).create();

                tx.send(repo).unwrap();
            });

            thread_handle.push(handle);
        }
        thread_handle
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .for_each(drop);
        drop(tx);

        // ---- Rayon impl - No Gain
        //let start_b = Instant::now();
        //let b = urls
        //    .into_par_iter()
        //    .map(|u| {
        //        let repo_config = RepositoryConfig {
        //            url: u,
        //            branchs: Vec::new(),
        //            all_branches,
        //        };
        //
        //        RepositoryFactory::with_config(repo_config).create()
        //    })
        //    .collect::<Vec<Repository>>();
        //println!("Duration with rayon : {:#?}", start_b.elapsed());
        rx.into_iter().collect::<Vec<Repository>>()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgCommitData {
    pub branches: HashMap<String, RepositoryCommitData>,
}

type RepoName = String;
impl CommittedDataExtraction<HashMap<RepoName, OrgCommitData>> for Org {
    fn committed_data(self) -> HashMap<RepoName, OrgCommitData> {
        let mut thread_handles = Vec::new();
        let (tx, rx) = mpsc::channel();

        for repository in self.repositories {
            let tx = mpsc::Sender::clone(&tx);

            let handle = thread::spawn(move || {
                let commited = repository.clone().committed_data();
                let user_commit_data = OrgCommitData { branches: commited };

                tx.send((repository.name, user_commit_data)).unwrap();
            });

            thread_handles.push(handle);
        }

        thread_handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .for_each(drop);
        drop(tx);

        rx.into_iter().collect::<HashMap<String, OrgCommitData>>()
    }
}
