use clap::ArgMatches;
use glit_core::config::GlobalConfig;

pub struct GlobalOptionHandler();

impl GlobalOptionHandler {
    pub fn config(matches: &ArgMatches) -> GlobalConfig {
        let verbose = matches
            .get_one::<bool>("verbose")
            .unwrap_or(&false)
            .to_owned();

        let output = matches
            .get_one::<String>("output")
            .unwrap_or(&"".to_string())
            .to_owned();

        GlobalConfig { verbose, output }
    }
}
