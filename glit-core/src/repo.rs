use std::{collections::BTreeMap, fs::remove_dir_all, path::PathBuf, str::FromStr};

use crate::{config::RepositoryConfig, log::Log, types::Branch, CommittedDataExtraction};
use ahash::HashMap;
use git2::{build::RepoBuilder, BranchType, Oid, RemoteConnection};
use rand::distributions::{Alphanumeric, DistString};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use reqwest::Url;
use serde::{Deserialize, Serialize};

const DEFAULT_PATH: &str = "/tmp";

#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    pub owner: String,
    pub url: Url,
    branches: Vec<Branch>,
    clone_paths: Vec<PathBuf>, // A local path (Folder) for each branch
}

pub struct RepositoryFactory {
    all_branches: bool,
    branches: Vec<Branch>,
    url: Url,
}

impl RepositoryFactory {
    pub fn with_config(repository_config: RepositoryConfig) -> Self {
        let url = repository_config.url;
        let all_branches: bool = repository_config.all_branches;

        RepositoryFactory {
            all_branches,
            url,
            branches: Vec::<Branch>::new(),
        }
    }

    fn clone(url: &Url, path: PathBuf) -> Result<git2::Repository, git2::Error> {
        RepoBuilder::new()
            .bare(true)
            .clone(url.as_str(), path.as_path())
    }

    fn clone_branches(url: Url, repo_name: String, branches: Vec<Branch>) -> Vec<PathBuf> {
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

                println!("Cloning branch : {:?} at {}", branch, path);
                RepoBuilder::new()
                    .bare(true)
                    .branch(&branch.to_string())
                    .clone(url.clone().as_str(), &branch_clone_path)
                    .unwrap();

                branch_clone_path
            })
            .collect::<Vec<PathBuf>>()
    }

    pub fn fetch_branches(repository: &git2::Repository) -> Vec<Branch> {
        let mut branches = repository
            .branches(Some(BranchType::Remote))
            .unwrap()
            .into_iter()
            .map(|b| {
                let branch = b.unwrap().0;
                let branch_name = branch.name().unwrap().unwrap();
                let string_branch = branch_name.split("origin/").last().unwrap().to_string();
                Branch(string_branch)
            })
            .collect::<Vec<Branch>>();

        let head = repository.head().unwrap();
        let head = head.name().unwrap();
        println!("Head Branch {}", head);

        branches.retain(|value| *value != Branch("HEAD".to_string()));
        branches.retain(|value| *value != Branch(head.to_string())); // Do not clone default branch two time

        branches
    }

    pub fn prepare_branch(branches: Vec<Branch>) -> Vec<Branch> {
        branches
            .iter()
            .map(|branch| Branch(branch.to_string().replace('/', "_")))
            .collect::<Vec<Branch>>()
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
            DEFAULT_PATH,
            hashed_repo_name,
            "main".to_string()
        ))
        .unwrap();

        let mut clone_paths: Vec<PathBuf> = Vec::new();
        let repo: git2::Repository = Self::clone(&self.url, clone_location.clone()).unwrap();

        // Clone all branches
        if self.all_branches {
            let branches = Self::fetch_branches(&repo);
            let prepared_branch = Self::prepare_branch(branches.clone());

            println!(
                "Building {} repository with branches {:?}",
                repo_name, prepared_branch
            );

            let paths = Self::clone_branches(self.url.clone(), repo_name.clone(), branches.clone());

            self.branches = branches;
            clone_paths.extend(paths);
        }
        // Clone only default branch
        else {
            clone_paths.push(clone_location);
            self.branches = vec![Branch("main".to_string())];
        }

        Repository {
            name: repo_name,
            owner,
            url: self.url.clone(),
            branches: self.branches.clone(),
            clone_paths,
        }
    }
}

type Mail = String; // A mail appear in a list of commit identified by a hash

// A Person commiting with his name and all the commit he pushed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Committer {
    pub mails: BTreeMap<Mail, Vec<String>>,
}

impl Committer {
    pub fn new(mail: String, commit_id: String) -> Self {
        let mut commits_for_mail = BTreeMap::new();
        commits_for_mail.insert(mail, vec![commit_id]);

        Self {
            mails: commits_for_mail,
        }
    }
}

//type AuthorName = String;
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AuthorName(pub String);

impl ToString for AuthorName {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepositoryCommitData {
    pub committers: BTreeMap<AuthorName, Committer>,
}

impl Default for RepositoryCommitData {
    fn default() -> Self {
        Self::new()
    }
}

impl RepositoryCommitData {
    pub fn new() -> Self {
        Self {
            committers: BTreeMap::<AuthorName, Committer>::new(),
        }
    }

    pub fn update(&mut self, repo: &git2::Repository, commit_id: Oid) -> Self {
        let commit = repo.find_commit(commit_id).unwrap();
        let commit_sigature = commit.author();
        let author: AuthorName = AuthorName(commit_sigature.name().unwrap_or("").to_string());
        let mail = commit_sigature.email().unwrap_or("").to_string();

        // To use only on first insertion
        let committer = Committer::new(mail.clone(), commit_id.to_string());

        if self.committers.contains_key(&author) {
            let mut existing_commiter = self.committers.get_mut(&author).unwrap().to_owned();

            if !existing_commiter.mails.contains_key(&mail) {
                existing_commiter.mails.insert(mail.clone(), vec![]);

                self.committers.insert(author.clone(), existing_commiter);
            }

            // Update commit_id list
            let mut actual_committer = self.committers.get_mut(&author).unwrap().to_owned();
            let mut commit_ids = actual_committer.mails.get_mut(&mail).unwrap().to_owned();

            commit_ids.push(commit_id.to_string());
            actual_committer.mails.insert(mail, commit_ids);

            // insert modified version of commiter
            self.committers.insert(author, actual_committer);
        } else {
            self.committers.insert(author.clone(), committer);
        }

        self.to_owned()
    }
}

impl CommittedDataExtraction<HashMap<String, RepositoryCommitData>> for Repository {
    fn committed_data(self) -> HashMap<String, RepositoryCommitData> {
        self.branches
            .clone()
            .into_iter()
            .zip(self.clone_paths)
            .map(|(br, pt)| {
                let repo_data = Log::build(pt.clone());
                remove_dir_all(pt.parent().unwrap()).unwrap();

                (br.to_string(), repo_data)
            })
            .collect::<HashMap<_, _>>()
    }
}
