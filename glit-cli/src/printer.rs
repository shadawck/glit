use colored::Colorize;
use glit_core::{config::GlobalConfig, org::Org, repo::Repository, user::User};
use std::marker::PhantomData;

pub struct Printer<T> {
    global_config: GlobalConfig,
    data: PhantomData<T>,
}

impl<T> Printer<T> {
    pub fn new(global_config: GlobalConfig) -> Self {
        Self {
            global_config,
            data: PhantomData::default(),
        }
    }
}

impl Printer<Repository> {
    pub fn print_repo(&self, data: &Repository) {
        if self.global_config.verbose {
        } else {
            for (branch, value) in &data.branch_data {
                let branch_format = format!("[ Branch : {} ]", branch.to_string()).yellow();
                println!("{}", branch_format);
                for (author, data) in &value.committers {
                    let mails = data.mails.keys().cloned().collect::<Vec<String>>();
                    print!("{}:", author.to_string().trim().blue());

                    print_mail(mails, author.to_string().trim());

                    println!();
                }
                println!();
            }
        }
    }
}

impl Printer<User> {
    pub fn print_user(&self, data: &User) {
        let printer = Printer::new(self.global_config.clone());
        for (repo_name, value) in data.repositories_data.clone() {
            let repo_format = format!("[ Repository : {} ]", repo_name.to_string()).magenta();
            println!("{}", repo_format);
            printer.print_repo(&value);
        }
    }
}
impl Printer<Org> {
    pub fn print_org(&self, data: &Org) {
        let printer = Printer::new(self.global_config.clone());
        for (repo_name, value) in data.repositories_data.clone() {
            let repo_format = format!("[ Repository : {} ]", repo_name.to_string()).magenta();
            println!("{}", repo_format);
            printer.print_repo(&value);
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
    if mail.contains("noreply.") {
        mail.red().to_string()
    } else {
        mail.green().to_string()
    }
}
