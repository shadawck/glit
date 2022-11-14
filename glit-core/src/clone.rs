use std::path::PathBuf;

// Cloned Repository on Local FileSytem
pub struct Clone {
    path: PathBuf,
}

impl Clone {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
