use crate::{
    config::RepositoryConfig,
    log::Log,
    types::{AuthorName, BranchName},
};
use ahash::{HashMap, HashMapExt};

use dashmap::DashMap;
use gix::{
    bstr::{BString, ByteSlice},
    progress::{self},
    remote::fetch::Shallow,
    Commit, ThreadSafeRepository,
};

use gitoxide::shared::pretty::prepare_and_run;
use gitoxide_core as core;

use rand::distributions::{Alphanumeric, DistString};
use rayon::{iter::IntoParallelIterator, prelude::ParallelIterator};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, remove_dir_all},
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};

const DEFAULT_PATH: &str = "/tmp";

#[derive(Debug, Clone)]
pub struct BranchData {
    pub branch_path: PathBuf,
    pub repo: ThreadSafeRepository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub url: String,
    pub name: String,
    pub owner: String,
    #[serde(skip)]
    pub repo_per_branches: DashMap<BranchName, BranchData>,
    pub branch_data: DashMap<BranchName, Committers>,
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

    fn get_default_local_branch(repo: &gix::ThreadSafeRepository) -> String {
        let binding = repo.to_thread_local();
        let references = binding.references().unwrap();

        let local = references
            .local_branches()
            .unwrap()
            .map(|a| a.unwrap().name().shorten().as_bstr().to_string())
            .collect::<Vec<String>>();

        local.get(0).unwrap().to_string()
    }

    pub fn find_remote_branches(
        repository: &gix::ThreadSafeRepository,
        default_local_branch: &str,
    ) -> Vec<BranchName> {
        let binding = repository.to_thread_local();
        let references = binding.references().unwrap();

        let mut remotes = references
            .remote_branches()
            .unwrap()
            .map(|origins| {
                let origins = origins.unwrap();
                let short_origin_name = origins.name().shorten();
                //println!("Short origin name {short_origin_name:#?}");
                BranchName(short_origin_name.to_string())
            })
            .collect::<Vec<BranchName>>();

        println!("All branches before retain : {:#?}", remotes);

        remotes.retain(|value| *value != BranchName("origin/HEAD".to_string()));
        remotes.retain(|value| *value != BranchName(default_local_branch.to_string())); // Do not clone default branch two time

        println!("All branches after retain : {:#?}", remotes);

        remotes
    }

    fn clone(url: reqwest::Url, path: PathBuf) -> gix::Repository {
        let urlx = gix::url::parse(url.as_str().into()).unwrap();
        println!("Url: {:?}", urlx.clone().to_bstring());

        let mut prepare_clone = gix::prepare_clone_bare(urlx.clone(), path.clone()).unwrap();
        //println!("Cloning {:?} into {:#?}...", urlx.to_bstring());

        let (prepare_checkout, _) = prepare_clone
            .fetch_then_checkout(progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .unwrap();

        println!("Clone ended");

        let repo = prepare_checkout.persist();
        println!(
            "default branch head commit id : {}",
            repo.head_commit().unwrap().id.to_string()
        );

        // CLONE TESTING
        let trace = false;
        let auto_verbose = true;
        let progress_keep_open = true;
        let progress = true;

        let config = vec![];
        let opts = core::repository::clone::Options {
            format: core::OutputFormat::Human,
            bare: false,
            handshake_info: true,
            no_tags: false,
            shallow: Shallow::NoChange,
        };

        let _ = prepare_and_run(
            "clone",
            trace,
            auto_verbose,
            progress,
            progress_keep_open,
            core::repository::clone::PROGRESS_RANGE,
            move |progress, out, err| {
                core::repository::clone(url.as_str(), Some(path), config, progress, out, err, opts)
            },
        );

        repo
    }

    fn clone_branches(
        url: Url,
        repo_name: String,
        branches: Vec<BranchName>,
    ) -> DashMap<BranchName, BranchData> {
        let repo_name = repo_name.replace('-', "_");

        let dash = DashMap::new();

        let _ = branches
            .into_iter()
            .map(|branch| {
                let hash_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 6);
                let hashed_repo_name = format!("{}_{}", repo_name, hash_suffix);

                let path = format!(
                    "{}/{}/{}",
                    DEFAULT_PATH,
                    hashed_repo_name,
                    branch.0.to_string()
                );

                let _ = fs::create_dir_all(path.clone());
                let branch_clone_path = PathBuf::from_str(&path).unwrap();
                let url = gix::url::parse(url.as_str().into()).unwrap();

                // TODO: can be obtained from the main branch path copy
                let prepare_clone =
                    gix::prepare_clone_bare(url.clone(), &branch_clone_path.clone()).unwrap();

                let branch_copy = branch.clone();
                let (prepare_checkout, _) = prepare_clone
                    .configure_remote(move |mut r| {
                        r.replace_refspecs(
                            [BString::from(branch_copy.to_string())],
                            gix::remote::Direction::Fetch,
                        )
                        .unwrap();

                        Ok(r)
                    })
                    .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
                    .unwrap();

                let repo = prepare_checkout.persist();

                let binding = branch.to_string();
                let binding = binding.split("/").collect::<Vec<&str>>();
                let short_name = binding.get(1).unwrap();

                //Self::fetch_repo(&repo, url, &short_name);
                //let oid = Self::checkout_worktree(&repo, &short_name, &branch_clone_path).unwrap();
                //
                //let local = repo
                //    .references()
                //    .unwrap()
                //    .local_branches()
                //    .unwrap()
                //    .map(|a| a.unwrap().name().shorten().as_bstr().to_string())
                //    .collect::<Vec<String>>();

                //local.get(0).unwrap().to_string();
                //println!("Local repo : {:#?}", local);
                //println!("Oid for {:#?} : {}", branch.to_string(), oid);

                let bd = BranchData {
                    branch_path: branch_clone_path,
                    repo: repo.into_sync(),
                };

                dash.insert(BranchName(short_name.to_string()), bd);
            })
            .collect::<()>();

        dash
    }

    //async fn async_clone_branches(
    //    url: Url,
    //    repo_name: String,
    //    branches: Vec<BranchName>,
    //) -> DashMap<BranchName, BranchData> {
    //    let repo_name = repo_name.replace('-', "_");
    //
    //    //let dash = Arc::new(DashMap::new());
    //    let dash = DashMap::new();
    //
    //    branches
    //        .iter()
    //        .map(|branch| async {
    //            //let branch = branch.clone();
    //            //let repo_name = repo_name.clone();
    //            //let url = url.clone();
    //            //let dash_arc = dash.clone();
    //
    //            let hash_suffix = Alphanumeric.sample_string(&mut rand::thread_rng(), 6);
    //            let hashed_repo_name = format!("{}_{}", repo_name, hash_suffix);
    //
    //            let path = format!(
    //                "{}/{}/{}",
    //                DEFAULT_PATH,
    //                hashed_repo_name,
    //                branch.0.to_string()
    //            );
    //
    //            let _ = fs::create_dir_all(path.clone());
    //            let branch_clone_path = PathBuf::from_str(&path).unwrap();
    //            let url = gix::url::parse(url.as_str().into()).unwrap();
    //
    //            //tokio::spawn(async move {
    //            // TODO: can be obtained from the main branch path copy
    //            let prepare_clone =
    //                gix::prepare_clone_bare(url.clone(), &branch_clone_path.clone()).unwrap();
    //
    //            let branch_copy = branch.clone();
    //            let (repo, _) = prepare_clone
    //                .configure_remote(move |mut r| {
    //                    r.replace_refspecs(
    //                        [BString::from(branch_copy.to_string())],
    //                        gix::remote::Direction::Fetch,
    //                    )
    //                    .unwrap();
    //
    //                    Ok(r)
    //                })
    //                .fetch_only(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
    //                .await
    //                .unwrap();
    //
    //            //let repo = prepare_checkout.persist();
    //
    //            let binding = branch.to_string();
    //            let binding = binding.split("/").collect::<Vec<&str>>();
    //            let short_name = binding.get(1).unwrap();
    //
    //            let bd = BranchData {
    //                branch_path: branch_clone_path,
    //                repo: Arc::new(Mutex::new(repo.into_sync())),
    //            };
    //
    //            dash.insert(BranchName(short_name.to_string()), bd);
    //            //})
    //            //.await;
    //        })
    //        .collect::<Vec<_>>();
    //
    //    dash
    //    //DashMap::new()
    //}

    pub fn create(self) -> Repository {
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

        fs::create_dir_all(clone_location.clone()).expect("Can't create directory for repository");

        // Clone default branch (main or master)
        let repo = Self::clone(self.url.clone(), clone_location.clone()).into_sync();
        let default_local_branch = Self::get_default_local_branch(&repo);

        let bd = BranchData {
            branch_path: clone_location.clone(),
            repo: repo.clone(),
        };
        let mut default_dash = DashMap::new();
        default_dash.insert(BranchName(default_local_branch.clone()), bd);

        // Clone all branches
        if self.all_branches {
            let branches = Self::find_remote_branches(&repo, &default_local_branch);

            if branches.last().unwrap().to_string() != format!("origin/{}", default_local_branch) {
                let all_branch_dash =
                    Self::clone_branches(self.url.clone(), repo_name.clone(), branches.clone());

                default_dash.extend(all_branch_dash);
            }
        }

        Repository {
            url: self.url.to_string(),
            name: repo_name,
            owner,
            repo_per_branches: default_dash,
            branch_data: DashMap::new(),
        }
    }
}

impl Repository {
    pub fn extract_log(mut self) -> Repository {
        self.branch_data = self
            .repo_per_branches
            .clone()
            .into_iter()
            .map(|(br, pt)| {
                let t1 = Instant::now();

                println!("handle {}", br.to_string());
                let repo_data = Log::build(br.clone(), &pt.clone(), &self.url);

                println!("Build log Time : {:?}", t1.elapsed());

                let remove_path = pt.branch_path.parent().unwrap();
                let removal = remove_dir_all(remove_path);
                match removal {
                    Ok(_) => log::debug!("Cleaning - Delete folder at {:?}", &remove_path),
                    Err(_) => log::error!("Failed to delete at {:?}", &remove_path),
                }

                (br, repo_data)
            })
            .collect::<DashMap<_, _>>();

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

    pub fn update(&mut self, commit: &Commit) -> &Self {
        log::debug!("Looking in commit {}", commit.id);

        let commit_sigature = commit.author().unwrap();
        let author: AuthorName = AuthorName(commit_sigature.name.as_bstr().to_string());
        let mail = commit_sigature.email.as_bstr().to_string();

        self.committers
            .entry(author)
            .and_modify(|committer| {
                // Author key exist. Need to modify it.
                committer
                    .mails
                    .entry(mail.clone())
                    .and_modify(|commit_ids| {
                        // Mail Key exist
                        commit_ids.push(commit.id.to_string());
                    })
                    .or_insert_with(||
                        // Mail Key do not exist
                        vec![commit.id.to_string()]);
            })
            .or_insert_with(||
                // Author Key do not exist
                Committer::new(mail, commit.id.to_string()));

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
