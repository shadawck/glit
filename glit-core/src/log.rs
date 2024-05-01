use crate::repo::Committers;
use git2::{Oid, Sort};
use indicatif::{ProgressBar, ProgressStyle};
use std::{path::PathBuf, thread};

pub struct Log {}

impl Log {
    pub fn build(path: PathBuf, repo_name: String, branch: String) -> Committers {
        let repo = git2::Repository::open_bare(path.as_path()).unwrap();
        let mut revwalk = repo.revwalk().unwrap();
        revwalk.set_sorting(Sort::TIME).unwrap();

        let mut repo_data = Committers::new();
        revwalk.push_head().unwrap();

        log::info!(
            "[{:?}][{:?}] Build log by revwalking ...",
            thread::current().id(),
            &path
        );

        let walk: Vec<Oid> = revwalk.map(|id| id.unwrap()).collect();

        let pb = ProgressBar::new(walk.len().try_into().unwrap());
        pb.set_message(format!("[{}][{}]", repo_name, branch));
        let style = ProgressStyle::with_template(
            "🏃 REVWALKING {msg}{spinner:.green}[{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ",
        )
        .unwrap()
        .progress_chars("#>-");

        pb.set_style(style);

        for (i, commit_id) in walk.into_iter().enumerate() {
            pb.set_position(i.try_into().unwrap());
            repo_data.update(&repo, commit_id);
        }

        pb.finish_with_message(format!("[{} ✅][{} ✅]", repo_name, branch));
        pb.finish_and_clear();
        repo_data
    }
}
