use reqwest::Url;

pub struct GlobalConfig {
    //pub with_proxy: bool,
    //with_format : Enum (json, txt)
    pub verbose: bool,
}

#[derive(Debug, Clone)]
pub struct RepositoryConfig {
    pub url: Url,
    pub branchs: Vec<String>,
    pub all_branches: bool,
}

pub struct UserConfig {
    pub url: Url,
    pub verbose: bool,
}

pub struct OrgConfig {
    pub url: Url,
    pub verbose: bool,
}
