use colored::Colorize;
use glit_core::{config::GlobalConfig, repo::RepositoryCommitData};
use serde_json;
use std::{collections::HashMap, fs, marker::PhantomData, path::PathBuf, str::FromStr};

pub struct Exporter<T> {
    global_config: GlobalConfig,
    _phantom_data: PhantomData<T>,
}

impl Exporter<HashMap<String, RepositoryCommitData>> {
    pub fn new(global_config: GlobalConfig) -> Self {
        Self {
            global_config,
            _phantom_data: PhantomData::default(),
        }
    }

    pub fn export(&self, data: &HashMap<String, RepositoryCommitData>) {
        let output = &self.global_config.output;

        if !output.is_empty() {
            let mut path = PathBuf::from_str(&output).unwrap();

            if path.is_dir() {
                path.set_file_name("repo.json");
            }

            let json_value = serde_json::to_string_pretty(data).unwrap();
            fs::write(path.as_path(), json_value).unwrap();

            println!("File -> {}", path.to_str().unwrap().purple());
        }
    }
}
