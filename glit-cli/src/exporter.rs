use colored::Colorize;
use glit_core::{config::GlobalConfig, org::Org, repo::Repository, user::User};
use serde_json;
use std::{fs, marker::PhantomData, path::PathBuf, str::FromStr};

pub struct Exporter<T> {
    global_config: GlobalConfig,
    _phantom_data: PhantomData<T>,
}

impl<T> Exporter<T> {
    pub fn new(global_config: GlobalConfig) -> Self {
        Self {
            global_config,
            _phantom_data: PhantomData,
        }
    }
}

impl Exporter<Repository> {
    pub fn export_repo(self, data: &Repository) {
        let output = self.global_config.output;

        if !output.is_empty() {
            let mut path = PathBuf::from_str(&output).unwrap();

            if path.is_dir() {
                path.set_file_name("repo.json");
            }

            let json_value = serde_json::to_string_pretty(&data.branch_data).unwrap();
            fs::write(path.as_path(), json_value).unwrap();

            println!("File -> {}", path.to_str().unwrap().yellow());
        }
    }
}

impl Exporter<User> {
    pub fn export_user(self, data: &User) {
        let output = self.global_config.output;

        if !output.is_empty() {
            let mut path = PathBuf::from_str(&output).unwrap();

            if path.is_dir() {
                path.set_file_name("user.json");
            }

            let json_value = serde_json::to_string_pretty(data).unwrap();
            fs::write(path.as_path(), json_value).unwrap();

            println!("File -> {}", path.to_str().unwrap().yellow());
        }
    }
}

impl Exporter<Org> {
    pub fn export_org(self, data: &Org) {
        let output = self.global_config.output;

        if !output.is_empty() {
            let mut path = PathBuf::from_str(&output).unwrap();

            if path.is_dir() {
                path.set_file_name("org.json");
            }

            let json_value = serde_json::to_string_pretty(data).unwrap();
            fs::write(path.as_path(), json_value).unwrap();

            println!("File -> {}", path.to_str().unwrap().yellow());
        }
    }
}
