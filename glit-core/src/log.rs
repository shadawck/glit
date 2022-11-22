use git2::{Oid, Sort};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
use tracing::{debug, info};

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
        let mut i = 0;

        let t1 = Instant::now();
        let mut t2 = Duration::new(0, 0);

        for commit_id in walk {
            if i % 100 == 0 {
                info!("Revwalk iteration {}/{} ", i, walk_iter_count);
            }
            i = i + 1;

            let t111 = Instant::now();
            repo_data.update(&repo, commit_id);
            t2 = t2 + t111.elapsed();
        }

        repo_data
    }
}
