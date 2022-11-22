use git2::{Oid, Sort};
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

        let walk: Vec<Oid> = revwalk.into_iter().map(|id| id.unwrap()).collect();
        let walk_iter_count = walk.len();

        for (i, commit_id) in walk.into_iter().enumerate() {
            if i % 1000 == 0 {
                info!("Revwalk iteration {}/{} ", i, walk_iter_count);
            }

            repo_data.update(&repo, commit_id);
        }

        repo_data
    }
}
