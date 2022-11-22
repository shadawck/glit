use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Branch(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Ord, Hash, PartialOrd, Serialize, Deserialize)]
pub struct AuthorName(pub String);

impl ToString for AuthorName {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoName(pub String);
impl ToString for RepoName {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(pub String);
impl ToString for BranchName {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mail(pub String);
impl ToString for Mail {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
