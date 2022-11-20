pub mod config;

pub mod log;
pub mod org;
pub mod repo;
pub mod types;
pub mod user;
pub trait ExtractLevel<T> {
    fn extract_log(self) -> T;
}
