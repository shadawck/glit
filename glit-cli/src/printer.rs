use std::{collections::HashMap, marker::PhantomData};

use colored::Colorize;
use glit_core::{
    config::GlobalConfig,
    org::OrgCommitData,
    repo::{Repository, RepositoryCommitData},
    user::UserCommitData,
};

pub struct Printer<T> {
    global_config: GlobalConfig,
    repo: Repository,
    data: PhantomData<T>,
}

type AuthorName = String;

impl Printer<HashMap<String, RepositoryCommitData>> {
    pub fn new(global_config: GlobalConfig, repo: Repository) -> Self {
        Self {
            global_config,
            repo,
            data: PhantomData::default(),
        }
    }

    pub fn with_repo(&mut self, repository: Repository) -> &Self {
        self.repo = repository;
        self
    }

    pub fn print(&self, data: &HashMap<AuthorName, RepositoryCommitData>) {
        println!("Check mail for {} of {}", self.repo.name, self.repo.owner);

        if self.global_config.verbose {
        } else {
            for (branch, value) in data {
                let branch_format = format!("[ Branch : {} ]", branch).yellow();
                println!("{}", branch_format);
                for (author, data) in &value.committer_data {
                    let mails = data.commit_list.keys().cloned().collect::<Vec<String>>();
                    print!("{}:", author.blue());

                    print_mail(mails, author);

                    println!("");
                }
                println!("");
            }
        }
    }
}

fn print_mail(mails: Vec<String>, author: &str) {
    if mails.len() == 1 {
        let mail = mails.first().unwrap().trim();
        let fmail = format_mail(mail);
        print!(" {}", fmail)
    } else {
        let author_string_len = author.len() + 1;
        let padding = " ".repeat(author_string_len);
        let mail = mails.first().unwrap().trim();
        let fmail = format_mail(mail);

        println!(" {}", fmail);
        for mail in mails[1..].to_vec() {
            let fmail = format_mail(mail.as_str());
            print!("{} {} ", padding, fmail);
        }
    }
}

fn format_mail(mail: &str) -> String {
    if mail.contains("noreply.github.com") {
        mail.red().to_string()
    } else {
        mail.green().to_string()
    }
}

impl Printer<UserCommitData> {
    pub fn print() {}
}
impl Printer<OrgCommitData> {
    pub fn print() {}
}
