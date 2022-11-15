use ahash::HashMap;
use std::marker::PhantomData;

use colored::Colorize;
use glit_core::{
    config::GlobalConfig, org::OrgCommitData, repo::RepositoryCommitData, user::UserCommitData,
};

pub struct Printer<T> {
    global_config: GlobalConfig,
    data: PhantomData<T>,
}

type AuthorName = String;
type RepoName = String;

impl<T> Printer<T> {
    pub fn new(global_config: GlobalConfig) -> Self {
        Self {
            global_config,
            data: PhantomData::default(),
        }
    }
}

impl Printer<HashMap<String, RepositoryCommitData>> {
    pub fn print_repo(&self, data: &HashMap<AuthorName, RepositoryCommitData>) {
        //println!("Check mail for {}", self.repo_name);

        if self.global_config.verbose {
        } else {
            for (branch, value) in data {
                let branch_format = format!("[ Branch : {} ]", branch).yellow();
                println!("{}", branch_format);
                for (author, data) in &value.committers {
                    let mails = data.commit_list.keys().cloned().collect::<Vec<String>>();
                    print!("{}:", author.trim().blue());

                    print_mail(mails, author.trim());

                    println!();
                }
                println!();
            }
        }
    }
}

impl Printer<UserCommitData> {
    pub fn print_user(&self, data: &HashMap<RepoName, UserCommitData>) {
        let printer = Printer::new(self.global_config.clone());
        for (repo_name, value) in data {
            let repo_format = format!("[ Repository : {} ]", repo_name).magenta();
            println!("{}", repo_format);
            printer.print_repo(&value.repositories_data);
        }
    }
}
impl Printer<OrgCommitData> {
    pub fn print_org(&self, data: &HashMap<RepoName, OrgCommitData>) {
        let printer = Printer::new(self.global_config.clone());
        for (repo_name, value) in data {
            let repo_format = format!("[ Repository : {} ]", repo_name).magenta();
            println!("{}", repo_format);
            printer.print_repo(&value.branches);
        }
    }
}

fn print_mail(mails: Vec<String>, author: &str) {
    if mails.len() == 1 {
        let mail = mails.first().unwrap().trim();
        let fmail = format_mail(mail);
        print!(" {}", fmail)
    } else {
        let author_string_len = author.len() + 2;
        let padding = " ".repeat(author_string_len);
        let mail = mails.first().unwrap().trim();
        let fmail = format_mail(mail);

        println!(" {}", fmail);
        for mail in mails[1..].iter() {
            let fmail = format_mail(mail.trim());
            print!("{}{}", padding, fmail);
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
