use std::path::PathBuf;

use crate::repo::RepositoryCommitData;

use git2::Sort;

pub struct Log {}

impl Log {
    pub fn build(path: PathBuf) -> RepositoryCommitData {
        let repo = git2::Repository::open(path.as_path()).unwrap();
        let mut revwalk = repo.revwalk().unwrap();
        revwalk.set_sorting(Sort::TIME).unwrap();

        let mut repo_data = RepositoryCommitData::new();

        revwalk.push_head().unwrap();

        println!("Build log for {:#?}", path);

        for commit_id in revwalk {
            let commit_id = commit_id.unwrap();

            repo_data.update(&repo, commit_id);
        }

        repo_data
    }
}
