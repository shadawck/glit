use git_repository::traverse::commit::Sorting;
use std::{path::PathBuf, thread};
use tracing::info;

use crate::repo::{Committers, Repository};

pub struct Log {}

impl Log {
    pub fn build(path: PathBuf) -> Committers {
        let repo = git_repository::open(path.as_path()).unwrap();
        let mut revwalk = repo.rev_walk(repo.head().unwrap().into_fully_peeled_id().unwrap());
        revwalk.sorting(Sorting::Topological);

        let mut repo_data = Committers::new();

        info!(
            "[{:?}][{:?}] Build log by revwalking",
            thread::current().id(),
            &path
        );
        let walk: Vec<git_repository::Id> = revwalk.all().unwrap().map(|id| id.unwrap()).collect();

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
