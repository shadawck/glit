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

        //print!("len rev : {:#?}", revwalk.into_iter().collect::<Vec<_>>());
        // Push commit head
        revwalk.push_head().unwrap();

        for commit_id in revwalk {
            let commit_id = commit_id.unwrap();

            // Build commit list

            repo_data.update(&repo, commit_id);

            //build commiter
            //Committer {
            //    name: author.to_string(),
            //    commit_list: ,
            //};
            //
            //// try to add commiter to list
            ////committer_seen.insert();
            //
            //let email = commit_sigature.email().unwrap();
        }
        //

        repo_data
    }
}
