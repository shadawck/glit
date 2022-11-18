pub mod config;

pub mod log;
pub mod org;
pub mod repo;
pub mod types;
pub mod user;
pub trait CommittedDataExtraction<T> {
    fn committed_data(self) -> T;
}
