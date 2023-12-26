use clap::ArgMatches;
use glit_core::config::GlobalConfig;

pub struct GlobalOptionHandler();

impl GlobalOptionHandler {
    pub fn config(matches: &ArgMatches) -> GlobalConfig {
        // TODO: get real  number of thread from hardware
        let thread_num = matches
            .get_one::<usize>("thread")
            .unwrap_or(&(8 as usize))
            .to_owned();

        let output = matches
            .get_one::<String>("output")
            .unwrap_or(&"".to_string())
            .to_owned();

        GlobalConfig { output, thread_num }
    }
}
