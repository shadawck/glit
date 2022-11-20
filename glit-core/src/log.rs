use git2::Sort;
use std::{fs::remove_dir_all, path::PathBuf, thread};

use crate::repo::Committers;

pub struct Log {}

impl Log {
    pub fn build(path: PathBuf) -> Committers {
        let repo = git2::Repository::open_bare(path.as_path()).unwrap();
        let mut revwalk = repo.revwalk().unwrap();
        revwalk.set_sorting(Sort::TIME).unwrap();

        let mut repo_data = Committers::new();

        revwalk.push_head().unwrap();

        println!(
            "Build log for {:#?} with thread ID : {:?}",
            path,
            thread::current().id()
        );

        for commit_id in revwalk {
            let commit_id = commit_id.unwrap();

            repo_data.update(&repo, commit_id);
        }

        remove_dir_all(path).unwrap();
        repo_data
    }
}
