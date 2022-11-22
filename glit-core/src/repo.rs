use crate::{
    config::RepositoryConfig,
    log::Log,
    types::{AuthorName, BranchName},
};
use ahash::{HashMap, HashMapExt};
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    BranchType, Oid,
};
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

use git2::{FetchOptions, Progress, RemoteCallbacks};

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

struct State {
    progress: Option<Progress<'static>>,
    total: usize,
    current: usize,
    path: Option<PathBuf>,
    newline: bool,
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

    fn get_head_branch(repo: &git2::Repository) -> String {
        let head = repo.head();
        if let Ok(head_ref) = head {
            head_ref
                .name()
                .unwrap()
                .split('/')
                .last()
                .unwrap()
                .to_string()
        } else {
            "".to_string()
        }
    }

    pub fn fetch_branches(repository: &git2::Repository, head: &str) -> Vec<BranchName> {
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

    fn clone(url: &Url, path: &Path) -> Result<git2::Repository, git2::Error> {
        let state = RefCell::new(State {
            progress: None,
            total: 0,
            current: 0,
            path: None,
            newline: false,
        });

        let mut cb = RemoteCallbacks::new();
        cb.transfer_progress(|stats| {
            let mut state = state.borrow_mut();

            state.progress = Some(stats.to_owned());
            print(&mut *state);
            true
        });

        let mut co = CheckoutBuilder::new();
        co.progress(|path, cur, total| {
            let mut state = state.borrow_mut();
            state.path = path.map(|p| p.to_path_buf());
            state.current = cur;
            state.total = total;
            print(&mut *state);
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);

        let repo = RepoBuilder::new()
            .bare(true)
            .fetch_options(fo)
            .with_checkout(co)
            .clone(url.as_str(), path);
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
                let state = RefCell::new(State {
                    progress: None,
                    total: 0,
                    current: 0,
                    path: None,
                    newline: false,
                });

                let mut cb = RemoteCallbacks::new();
                cb.transfer_progress(|stats| {
                    let mut state = state.borrow_mut();

                    state.progress = Some(stats.to_owned());
                    print(&mut *state);
                    true
                });

                let mut co = CheckoutBuilder::new();
                co.progress(|path, cur, total| {
                    let mut state = state.borrow_mut();
                    state.path = path.map(|p| p.to_path_buf());
                    state.current = cur;
                    state.total = total;
                    print(&mut *state);
                });

                let mut fo = FetchOptions::new();
                fo.remote_callbacks(cb);

                let repo = RepoBuilder::new()
                    .bare(true)
                    .fetch_options(fo)
                    .with_checkout(co)
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
        let repo: git2::Repository = Self::clone(&self.url, clone_location.as_path()).unwrap();
        let head = Self::get_head_branch(&repo);
        if !head.is_empty() {
            clone_paths.push(clone_location);
        }

        // Clone all branches
        if self.all_branches {
            let mut branches = Self::fetch_branches(&repo, &head);
            //let prepared_branch = Self::prepare_branch(branches.clone());

            let paths = Self::clone_branches(self.url.clone(), repo_name.clone(), branches.clone());

            branches.push(BranchName(head));
            self.branches = branches.clone();

            clone_paths.extend(paths);
        }
        // Clone only default branch
        else {
            self.branches = vec![BranchName(head)];
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
                let removal = remove_dir_all(&remove_path);
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

    pub fn update(&mut self, repo: &git2::Repository, commit_id: Oid) -> &Self {
        let commit = repo.find_commit(commit_id).unwrap();
        let commit_sigature = commit.author();
        let author: AuthorName = AuthorName(commit_sigature.name().unwrap_or("").to_string());
        let mail = commit_sigature.email().unwrap_or("").to_string();

        self.committers
            .entry(author)
            .and_modify(|committer| {
                // Author key exist. Need to modify it.
                committer
                    .mails
                    .entry(mail.clone())
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

        // A little bit faster but not cleaner
        //let committer = Committer::new(mail.clone(), commit_id.to_string());
        //if self.committers.contains_key(&author) {
        //    let mut existing_commiter = self.committers.get_mut(&author).unwrap().to_owned();
        //
        //    if !existing_commiter.mails.contains_key(&mail) {
        //        existing_commiter.mails.insert(mail.clone(), vec![]);
        //
        //        self.committers.insert(author.clone(), existing_commiter);
        //    }
        //
        //    // Update commit_id list
        //    let mut actual_committer = self.committers.get_mut(&author).unwrap().to_owned();
        //    let mut commit_ids = actual_committer.mails.get_mut(&mail).unwrap().to_owned();
        //
        //    commit_ids.push(commit_id.to_string());
        //    actual_committer.mails.insert(mail, commit_ids);
        //
        //    // insert modified version of commiter
        //    self.committers.insert(author, actual_committer);
        //} else {
        //    self.committers.insert(author.clone(), committer);
        //}

        self
    }
}

fn print(state: &mut State) {
    let stats = state.progress.as_ref().unwrap();
    let network_pct = (100 * stats.received_objects()) / stats.total_objects();
    let index_pct = (100 * stats.indexed_objects()) / stats.total_objects();
    let co_pct = if state.total > 0 {
        (100 * state.current) / state.total
    } else {
        0
    };
    let kbytes = stats.received_bytes() / 1024;
    if stats.received_objects() == stats.total_objects() {
        if !state.newline {
            println!();
            state.newline = true;
        }
        print!(
            "Resolving deltas {}/{}\r",
            stats.indexed_deltas(),
            stats.total_deltas()
        );
    } else {
        print!(
            "net {:3}% ({:4} kb, {:5}/{:5})  /  idx {:3}% ({:5}/{:5})  \
             /  chk {:3}% ({:4}/{:4}) {}\r",
            network_pct,
            kbytes,
            stats.received_objects(),
            stats.total_objects(),
            index_pct,
            stats.indexed_objects(),
            stats.total_objects(),
            co_pct,
            state.current,
            state.total,
            state
                .path
                .as_ref()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        )
    }
    io::stdout().flush().unwrap();
}
