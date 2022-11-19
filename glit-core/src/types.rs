use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Branch(pub String);

impl ToString for Branch {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
