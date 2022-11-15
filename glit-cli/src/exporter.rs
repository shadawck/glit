use colored::Colorize;

use ahash::HashMap;
use glit_core::{
    config::GlobalConfig, org::OrgCommitData, repo::RepositoryCommitData, user::UserCommitData,
};
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
            _phantom_data: PhantomData::default(),
        }
    }
}

impl Exporter<HashMap<String, RepositoryCommitData>> {
    pub fn export_repo(self, data: &HashMap<String, RepositoryCommitData>) {
        let output = self.global_config.output;

        if !output.is_empty() {
            let mut path = PathBuf::from_str(&output).unwrap();

            if path.is_dir() {
                path.set_file_name("repo.json");
            }

            let json_value = serde_json::to_string_pretty(data).unwrap();
            fs::write(path.as_path(), json_value).unwrap();

            println!("File -> {}", path.to_str().unwrap().yellow());
        }
    }
}

impl Exporter<HashMap<String, UserCommitData>> {
    pub fn export_user(self, data: &HashMap<String, UserCommitData>) {
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

impl Exporter<HashMap<String, OrgCommitData>> {
    pub fn export_org(self, data: &HashMap<String, OrgCommitData>) {
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
