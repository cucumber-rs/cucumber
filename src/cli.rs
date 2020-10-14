use clap::{App, Arg};

#[derive(Default)]
pub struct CliOptions {
    pub scenario_filter: Option<String>,
    pub nocapture: bool,
}

pub fn make_app() -> CliOptions {
    let matches = App::new("cucumber")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Brendan Molloy <brendan@bbqsrc.net>")
        .about("Run the tests, pet a dog!")
        .arg(
            Arg::with_name("filter")
                .short("e")
                .long("expression")
                .value_name("regex")
                .help("Regex to select scenarios from")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("nocapture")
                .long("nocapture")
                .help("Use this flag to disable suppression of output from tests"),
        )
        .get_matches();

    let nocapture = matches.is_present("nocapture");
    let scenario_filter = matches.value_of("filter").map(|v| v.to_string());

    CliOptions {
        nocapture,
        scenario_filter,
    }
}
