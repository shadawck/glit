use crate::{
    config::RepositoryConfig,
    log::Log,
    types::{AuthorName, BranchName},
};
use ahash::{HashMap, HashMapExt};
use rand::distributions::{Alphanumeric, DistString};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::remove_dir_all,
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};
use tracing::{error, info};

use git_repository::{bstr::ByteSlice, open, open_opts, prelude::*, remote::Direction};

const DEFAULT_PATH: &str = "/tmp";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub owner: String,
    branches: Vec<BranchName>,
    #[serde(skip)]
    clone_paths: Vec<PathBuf>,
    pub branch_data: HashMap<BranchName, Committers>,
}

pub struct RepositoryFactory {
    all_branches: bool,
    branches: Vec<BranchName>,
    url: Url,
}

impl RepositoryFactory {
    pub fn with_config(repository_config: RepositoryConfig) -> Self {
        let url = repository_config.url;
        let all_branches: bool = repository_config.all_branches;

        RepositoryFactory {
            all_branches,
            url,
            branches: Vec::<BranchName>::new(),
        }
    }

    //fn get_head_branch(repo: Repository) -> String {
    //    let head = repo.head();
    //    if let Ok(head_ref) = head {
    //        head_ref
    //            .name()
    //            .unwrap()
    //            .split('/')
    //            .last()
    //            .unwrap()
    //            .to_string()
    //    } else {
    //        "".to_string()
    //    }
    //}

    pub fn fetch_branches(repository: &git_repository::Repository, head: &str) -> Vec<BranchName> {
        let default_remote = repository.remote_default_name(Direction::Fetch).unwrap();
        println!("THE DEFAULT REMOTE : {}", default_remote);

        let mut branches = repository
            .branches(Some(BranchType::Remote))
            .unwrap()
            .into_iter()
            .map(|b| {
                let branch = b.unwrap().0;
                let branch_name = branch.name().unwrap().unwrap();
                let string_branch = branch_name.split("origin/").last().unwrap().to_string();
                BranchName(string_branch)
            })
            .collect::<Vec<_>>();

        branches.retain(|value| *value != BranchName("HEAD".to_string()));
        branches.retain(|value| *value != BranchName(head.to_string())); // Do not clone default branch two time

        branches
    }

    pub fn prepare_branch(branches: Vec<BranchName>) -> Vec<BranchName> {
        branches
            .iter()
            .map(|branch| BranchName(branch.to_string().replace('/', "_")))
            .collect::<Vec<_>>()
    }

    fn clone(url: &str, path: &Path) -> git_repository::Repository {
        let open_opts = git_repository::open::Options::default();
        let create_opts = git_repository::create::Options::default();

        let repo: git_repository::Repository = git_repository::clone::PrepareFetch::new(
            url,
            path,
            git_repository::create::Kind::Bare,
            create_opts,
            open_opts,
        )
        .unwrap()
        .persist();

        repo
    }

    fn clone_branches(url: Url, repo_name: String, branches: Vec<BranchName>) -> Vec<PathBuf> {
        let repo_name = repo_name.replace('-', "_");
        branches
            .par_iter()
            .map(|branch| {
                let hash_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 6);
                let hashed_repo_name = format!("{}_{}", repo_name, hash_suffix);

                let path = format!(
                    "{}/{}/{}",
                    DEFAULT_PATH,
                    hashed_repo_name,
                    branch.to_string(),
                );

                let branch_clone_path = PathBuf::from_str(&path).unwrap();

                let repo = RepoBuilder::new()
                    .bare(true)
                    .branch(&branch.to_string())
                    .clone(url.clone().as_str(), &branch_clone_path);

                match repo {
                    Ok(_) => info!(
                        "[{:?}] Cloning branch : {:?} at {}",
                        repo_name, branch, path
                    ),
                    Err(_) => info!("Failed to clone {} with branch {:?}", repo_name, branch),
                }

                branch_clone_path
            })
            .collect::<Vec<PathBuf>>()
    }

    pub fn create(mut self) -> Repository {
        let mut path_segments = self.url.path_segments().unwrap();
        let owner = path_segments.next().unwrap().to_string();
        let repo_name = path_segments.next().unwrap().to_string();

        // default location
        let hash_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 6);
        let hashed_repo_name = format!("{}_{}", repo_name, hash_suffix);
        let clone_location = PathBuf::from_str(&format!(
            "{}/{}/{}",
            DEFAULT_PATH, hashed_repo_name, "default"
        ))
        .unwrap();

        let mut clone_paths: Vec<PathBuf> = Vec::new();
        let repo: git_repository::Repository =
            Self::clone(&self.url.as_str(), clone_location.as_path());

        let head = repo.head().unwrap();
        let head_name = head.name().as_bstr().to_string();
        println!("HEAD NAME {}", head_name);

        if !head.is_unborn() {
            clone_paths.push(clone_location);
        }

        // Clone all branches
        if self.all_branches {
            let mut branches = Self::fetch_branches(&repo, &head_name);
            //let prepared_branch = Self::prepare_branch(branches.clone());

            let paths = Self::clone_branches(self.url.clone(), repo_name.clone(), branches.clone());

            branches.push(BranchName(head_name));
            self.branches = branches.clone();

            clone_paths.extend(paths);
        }
        // Clone only default branch
        else {
            self.branches = vec![BranchName(head_name)];
        }

        Repository {
            name: repo_name,
            owner,
            branches: self.branches.clone(),
            clone_paths,
            branch_data: HashMap::new(),
        }
    }
}

impl Repository {
    pub fn extract_log(mut self) -> Repository {
        self.branch_data = self
            .branches
            .clone()
            .into_iter()
            .zip(self.clone_paths.clone())
            .map(|(br, pt)| {
                let t1 = Instant::now();
                let repo_data = Log::build(pt.clone());
                println!("Build log Time : {:?}", t1.elapsed());

                let remove_path = pt.parent().unwrap();
                let removal = remove_dir_all(remove_path);
                match removal {
                    Ok(_) => info!("Cleaning - Delete folder at {:?}", &remove_path),
                    Err(_) => error!("Failed to delete at {:?}", &remove_path),
                }

                (br, repo_data)
            })
            .collect::<HashMap<_, _>>();

        self
    }
}

type Mail = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Committer {
    pub mails: BTreeMap<Mail, Vec<Mail>>,
}

impl Committer {
    pub fn new(mail: Mail, commit_id: String) -> Self {
        let mut commits_for_mail = BTreeMap::new();
        commits_for_mail.insert(mail, vec![commit_id]);

        Self {
            mails: commits_for_mail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Committers {
    pub committers: HashMap<AuthorName, Committer>,
}

impl Default for Committers {
    fn default() -> Self {
        Self::new()
    }
}

impl Committers {
    pub fn new() -> Self {
        Self {
            committers: HashMap::<AuthorName, Committer>::new(),
        }
    }

    pub fn update(
        &mut self,
        repo: &git_repository::Repository,
        commit_id: git_repository::Id,
    ) -> &Self {
        let commit = repo.find_object(commit_id).unwrap();
        println!("COMMIT OBJECT : {:?}", commit);

        let (author, mail) = commit.into_commit().author().unwrap().actor();
        let author: AuthorName = AuthorName(author.to_string());
        let mail = mail.to_string();

        println!("mail : {}/ author : {:?}", mail, author);

        self.committers
            .entry(author)
            .and_modify(|committer| {
                // Author key exist. Need to modify it.
                committer
                    .mails
                    .entry(mail.clone().to_string())
                    .and_modify(|commit_ids| {
                        // Mail Key exist
                        commit_ids.push(commit_id.to_string());
                    })
                    .or_insert_with(||
                        // Mail Key do not exist
                        vec![commit_id.to_string()]);
            })
            .or_insert_with(||
                // Author Key do not exist
                Committer::new(mail, commit_id.to_string()));

        self
    }
}
