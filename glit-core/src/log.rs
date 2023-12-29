use crate::{
    repo::{BranchData, Committers},
    types::BranchName,
};
//use async_std::sync::Mutex;
use gix::{
    bstr::{BString, ByteSlice},
    odb::FindExt,
    progress::Discard,
    refs::{
        transaction::{Change, LogChange, RefEdit},
        FullName, Target,
    },
    remote::{fetch::Status, ref_map::Options, Direction},
    Commit, ObjectId, Remote, ThreadSafeRepository,
};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    thread,
};
//use tokio::sync::Mutex;

pub struct Log;

impl Log {
    fn fetch_repo(repo: ThreadSafeRepository, url: gix::Url, branch_name: &str) {
        let repo = repo.to_thread_local();
        let outcome = repo
            .remote_at(url)
            .unwrap()
            .with_refspecs([BString::from(branch_name)], Direction::Fetch)
            .unwrap()
            .connect(Direction::Fetch)
            .unwrap()
            .prepare_fetch(Discard, Options::default())
            .unwrap()
            .receive(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .unwrap();

        if let Status::Change { .. } = outcome.status {
            let needle = BString::from("refs/heads/".to_owned() + branch_name);
            let target = outcome
                .ref_map
                .mappings
                .iter()
                .find(|m| m.remote.as_name() == Some(needle.as_bstr()))
                .unwrap()
                .remote
                .as_id()
                .unwrap()
                .to_owned();

            let edit = RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: gix::refs::transaction::RefLog::AndReference,
                        force_create_reflog: false,
                        message: BString::from("fetch branch"),
                    },
                    expected: gix::refs::transaction::PreviousValue::Any,
                    new: Target::Peeled(target),
                },
                name: FullName::try_from(needle).unwrap(),
                deref: false,
            };
            repo.edit_reference(edit).unwrap();
        }
    }

    fn checkout_worktree(
        repo: ThreadSafeRepository,
        branch: &str,
        workdir: &Path,
    ) -> Result<ObjectId, ()> {
        let repo = repo.to_thread_local();

        let local = repo
            .references()
            .unwrap()
            .local_branches()
            .unwrap()
            .map(|a| a.unwrap().name().shorten().as_bstr().to_string())
            .collect::<Vec<String>>();

        //println!("Local : {:#?}", local);

        local.get(0).unwrap().to_string();

        let oid = repo
            .refs
            .find(branch)
            .unwrap()
            .target
            .try_into_id()
            .unwrap();

        let tree_id = repo
            .find_object(oid)
            .unwrap()
            .into_commit()
            .tree_id()
            .unwrap();
        let (mut state, _) = repo.index_from_tree(&tree_id).unwrap().into_parts();

        let odb = repo.objects.clone().into_arc().unwrap();

        let _outcome = gix::worktree::state::checkout(
            &mut state,
            workdir,
            move |oid, buf| odb.find_blob(oid, buf),
            &mut Discard,
            &mut Discard,
            &AtomicBool::default(),
            gix::worktree::state::checkout::Options::default(),
        )
        .unwrap();

        Ok(oid)
    }

    pub fn build(branch: BranchName, bd: &BranchData, url: &str) -> Committers {
        // GIX
        let repo = &bd.repo;
        let mut repo_data_gix = Committers::new();
        let url = gix::url::parse(url.into()).unwrap();

        let binding = branch.to_string();
        let binding = binding.split("/").collect::<Vec<&str>>();
        let short_name = binding.last().unwrap();

        println!("Fetching repo");
        Self::fetch_repo(repo.clone(), url, &short_name);
        println!("Checking repo");
        let oid = Self::checkout_worktree(repo.clone(), &short_name, &bd.branch_path).unwrap();

        //let local = repo
        //    .references()
        //    .unwrap()
        //    .local_branches()
        //    .unwrap()
        //    .map(|a| a.unwrap().name().shorten().as_bstr().to_string())
        //    .collect::<Vec<String>>();
        //
        //local.get(0).unwrap().to_string();
        //println!("Local repo : {:#?}", local);
        //println!("Oid for {:#?} : {}", branch.to_string(), oid);
        //println!("HEAD in log : {} for branch {}", oid, branch.to_string());
        //let oid = repo.head_id().unwrap();

        let repo = repo.to_thread_local();
        let rev_walk = repo.rev_walk(Some(oid));
        let commits = rev_walk
            .use_commit_graph(false)
            .all()
            .unwrap()
            .map(|a| a.unwrap().object().unwrap())
            .collect::<Vec<Commit>>();

        for obj in commits {
            repo_data_gix.update(&obj);
        }

        repo_data_gix
    }
}
