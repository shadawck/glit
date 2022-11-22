use reqwest::Url;

#[derive(Debug, Clone)]
pub struct GlobalConfig {
    pub output: String,
    pub verbose: bool,
}

#[derive(Debug, Clone)]
pub struct RepositoryConfig {
    pub url: Url,
    pub all_branches: bool,
}

impl RepositoryConfig {
    pub fn new(url: Url, all_branches: bool) -> Self {
        Self { url, all_branches }
    }
}

#[derive(Debug, Clone)]
pub struct UserConfig {
    pub url: Url,
    pub all_branches: bool,
}

#[derive(Debug, Clone)]
pub struct OrgConfig {
    pub url: Url,
    pub all_branches: bool,
}
