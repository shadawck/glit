use std::{
    collections::BTreeMap,
    collections::HashMap,
    fs::remove_dir_all,
    path::{Path, PathBuf},
    str::FromStr,
    thread,
};

use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};

use std::sync::mpsc;

use git2::{build::RepoBuilder, BranchType, Oid};
use reqwest::Url;

use crate::{config::RepositoryConfig, log::Log, CommittedDataExtraction};

const DEFAULT_PATH: &str = "/tmp";

#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    pub owner: String,
    pub url: Url,
    branches: Vec<String>,
    clone_paths: Vec<PathBuf>, // A path for each branch
}

pub struct RepositoryFactory {
    branches: Vec<String>,
    all_branches: bool,
    url: Url,
}

impl RepositoryFactory {
    pub fn with_config(repository_config: RepositoryConfig) -> Self {
        let branches = repository_config.branchs;
        let url = repository_config.url;
        let all_branches: bool = repository_config.all_branches;

        RepositoryFactory {
            branches,
            all_branches,
            url,
        }
    }

    fn clone(url: Url, path: PathBuf) -> Result<git2::Repository, git2::Error> {
        RepoBuilder::new()
            .bare(true)
            .clone(url.as_str(), &path.as_path())
    }

    fn clone_multiple_branches(url: Url, name: String, branches: Vec<String>) -> Vec<PathBuf> {
        let name = name.replace("-", "_");
        // Make it multithreaded
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::new();

        for branch in branches {
            let name = name.clone();
            let url = url.clone();
            let tx = mpsc::Sender::clone(&tx);

            let handle = thread::spawn(move || {
                let hash_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
                let path = format!(
                    "{}/{}_{}_{}",
                    DEFAULT_PATH,
                    name.clone(),
                    branch.replace("/", "_").as_str(),
                    hash_suffix
                );
                let location = Path::new(&path);

                //println!("Cloning branch : {} at {}", branch, path);
                RepoBuilder::new()
                    .bare(true)
                    .branch(&branch)
                    .clone(url.clone().as_str(), location.clone())
                    .unwrap();

                tx.send(location.to_path_buf()).unwrap();
            });

            handles.push(handle);
        }
        drop(tx);

        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .for_each(drop);

        rx.into_iter().collect::<Vec<PathBuf>>()
    }

    pub fn create(mut self) -> Repository {
        let mut path_segments = self.url.path_segments().unwrap();
        let owner = path_segments.next().unwrap().to_string();

        let name = path_segments.next().unwrap().to_string();
        println!("Building {} repository...", name);

        let url = self.url;

        // default location
        let hash_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
        let clone_location =
            PathBuf::from_str(&format!("{}/{}_{}", DEFAULT_PATH, name, hash_suffix)).unwrap();
        // Always clone default (main/master branch)
        let repo = Self::clone(url.clone(), clone_location.clone()).unwrap();

        let mut clone_paths: Vec<PathBuf> = Vec::new();

        if self.branches.is_empty() {
            // Select all branch
            if self.all_branches.eq(&true) {
                let branches = repo
                    .branches(Some(BranchType::Remote))
                    .unwrap()
                    .into_iter()
                    .map(|b| {
                        let branch = b.unwrap().0;
                        branch
                            .name()
                            .unwrap()
                            .unwrap()
                            .split("/")
                            .last()
                            .unwrap()
                            .to_string()
                    })
                    .collect::<Vec<String>>()[1..]
                    .to_vec();

                println!(" with branches {:?}", branches);

                let new_paths =
                    Self::clone_multiple_branches(url.clone(), name.clone(), branches.clone());

                self.branches = branches;
                clone_paths.extend(new_paths);
                remove_dir_all(clone_location).unwrap();
            } else {
                clone_paths.push(clone_location);
            }
        } else {
            // Select multiple branch (User)
            println!("user selection");
            let new_paths =
                Self::clone_multiple_branches(url.clone(), name.clone(), self.branches.clone());
            clone_paths.extend(new_paths);
            remove_dir_all(clone_location).unwrap();
        }

        Repository {
            name,
            owner,
            url,
            branches: self.branches,
            clone_paths,
        }
    }
}

type Mail = String; // A mail appear ...
                    // ... in a list of commit identified by a hash

// A Person commiting with his name and all the commit he pushed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Committer {
    pub commit_list: BTreeMap<Mail, Vec<String>>,
}

impl Committer {
    pub fn new(mail: String, commit_id: String) -> Self {
        let mut commit_list = BTreeMap::new();
        commit_list.insert(mail, vec![commit_id]);

        Self { commit_list }
    }
}

type AuthorName = String;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RepositoryCommitData {
    pub committer_data: BTreeMap<AuthorName, Committer>,
}

impl RepositoryCommitData {
    pub fn new() -> Self {
        Self {
            committer_data: BTreeMap::<AuthorName, Committer>::new(),
        }
    }

    pub fn update(&mut self, repo: &git2::Repository, commit_id: Oid) -> Self {
        let commit = repo.find_commit(commit_id).unwrap();
        let commit_sigature = commit.author();
        let author: AuthorName = commit_sigature.name().unwrap_or("").to_string();
        let mail = commit_sigature.email().unwrap_or("").to_string();

        // To use only on first insertion
        let committer = Committer::new(mail.clone(), commit_id.to_string());

        if self.committer_data.contains_key(&author) {
            let mut existing_commiter = self.committer_data.get_mut(&author).unwrap().to_owned();

            if !existing_commiter.commit_list.contains_key(&mail) {
                existing_commiter.commit_list.insert(mail.clone(), vec![]);

                self.committer_data
                    .insert(author.clone(), existing_commiter);
            }

            // Update commit_id list
            let mut actual_committer = self.committer_data.get_mut(&author).unwrap().to_owned();
            let mut commit_ids = actual_committer
                .commit_list
                .get_mut(&mail)
                .unwrap()
                .to_owned();

            commit_ids.push(commit_id.to_string());
            actual_committer.commit_list.insert(mail, commit_ids);

            // insert modified version of commiter
            //println!("Insert new version of committer");
            self.committer_data.insert(author, actual_committer);
        } else {
            self.committer_data.insert(author.clone(), committer);
            //println!("The author {} have been added", author.clone());
        }

        self.to_owned()
    }
}

impl CommittedDataExtraction<HashMap<String, RepositoryCommitData>> for Repository {
    fn committed_data(self) -> HashMap<String, RepositoryCommitData> {
        let mut handles = vec![];

        let (tx, rx) = mpsc::channel();

        if self.clone_paths.len().eq(&1) {
            let path = self.clone_paths.first().unwrap();
            let branch = self
                .branches
                .first()
                .unwrap_or(&"Default (Master, Main)".to_string())
                .to_owned();
            let repo_data = Log::build(path.to_path_buf());

            tx.send((branch, repo_data)).unwrap();

            remove_dir_all(self.clone_paths.first().unwrap()).unwrap();
        } else {
            //println!("{} {}", self.branches.len(), self.clone_paths.len());

            for val in self.branches.clone().into_iter().zip(self.clone_paths) {
                let (br, pt) = val.clone();
                let tx = mpsc::Sender::clone(&tx);

                let handle = thread::spawn(move || {
                    //println!("Gathering data on branch : {} at {:?}", br, pt);

                    // log function
                    let repo_data = Log::build(pt.clone());
                    tx.send((br, repo_data)).unwrap();

                    // Cleanup
                    remove_dir_all(pt).unwrap();
                });

                handles.push(handle);
            }

            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .for_each(drop);
        }
        drop(tx);

        rx.into_iter()
            .collect::<HashMap<String, RepositoryCommitData>>()
    }
}
