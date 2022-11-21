use git2::Sort;
use std::{path::PathBuf, thread};
use tracing::info;

use crate::repo::Committers;

pub struct Log {}

impl Log {
    pub fn build(path: PathBuf) -> Committers {
        let repo = git2::Repository::open_bare(path.as_path()).unwrap();
        let mut revwalk = repo.revwalk().unwrap();
        revwalk.set_sorting(Sort::TIME).unwrap();

        let mut repo_data = Committers::new();

        revwalk.push_head().unwrap();

        info!(
            "[{:?}][{:?}] Build log by revwalking",
            thread::current().id(),
            &path
        );

        for commit_id in revwalk {
            let commit_id = commit_id.unwrap();

            repo_data.update(&repo, commit_id);
        }
        repo_data
    }
}
