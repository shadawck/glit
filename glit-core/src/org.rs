use std::{sync::mpsc, thread};

use ahash::HashMap;
use crossbeam::channel::{bounded, unbounded};
//use std::collections::HashMap;
use futures::future::join_all;
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
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

        println!("{}", repository_count_str);

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

        println!("page_url : {:#?}", pages_urls);

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

        let (tx, rx) = unbounded();
        println!("Bounded created");

        for page in pages_urls {
            println!("Entering loop");
            let client = client.clone();
            let tx = tx.clone();

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
                        tx.send(endpoint_url).unwrap();
                        drop(&tx)
                    })
                    .for_each(drop)
            });

            tokio_handles.push(handle);
        }
        join_all(tokio_handles).await;
        drop(tx);

        // One options is to just recv a repository and pass it to top calling function without waiting for
        // for all the url to be fetch.
        let urls = rx.iter().collect::<Vec<String>>(); // <--- Blocking / Barrier

        // ---- Rayon impl
        urls.par_iter()
            .map(|endpoint_url| {
                let repo_name = endpoint_url.split('/').last().unwrap();
                let repo_url = format!("{}{}/", base_url, repo_name);
                let url = Url::parse(&repo_url).unwrap();

                let repo_config = RepositoryConfig {
                    url,
                    branchs: Vec::new(),
                    all_branches,
                };

                // Github Cloning operation
                RepositoryFactory::with_config(repo_config).create()
            })
            .collect::<Vec<Repository>>()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgCommitData {
    pub branches: HashMap<String, RepositoryCommitData>,
}

type RepoName = String;
impl CommittedDataExtraction<HashMap<RepoName, OrgCommitData>> for Org {
    fn committed_data(self) -> HashMap<RepoName, OrgCommitData> {
        self.repositories
            .par_iter()
            .map(|repo| {
                let branches_data = repo.to_owned().committed_data();
                let org_commit_data = OrgCommitData {
                    branches: branches_data,
                };
                (repo.name.to_owned(), org_commit_data)
            })
            .collect::<HashMap<_, _>>()
    }
}
