use clap::{App, Arg};
use regex::Regex;

#[derive(Debug)]
pub enum CliError {
    InvalidFilterRegex,
}

#[derive(Default)]
pub struct CliOptions {
    pub feature: Option<String>,
    pub filter: Option<Regex>,
    pub tag: Option<String>,
    pub suppress_output: bool,
}

pub fn make_app() -> Result<CliOptions, CliError> {
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
            Arg::with_name("feature")
                .short("f")
                .long("feature")
                .value_name("feature")
                .help("Specific feature file(s) to use with a glob (optional)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("tag")
                .short("t")
                .long("tag")
                .value_name("tag")
                .help("Filter by specified tag")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("nocapture")
                .long("nocapture")
                .help("Use this flag to disable suppression of output from tests"),
        )
        .get_matches();

    let filter = if let Some(filter) = matches.value_of("filter") {
        let regex = Regex::new(filter).map_err(|_| CliError::InvalidFilterRegex)?;
        Some(regex)
    } else {
        None
    };

    let feature = matches.value_of("feature").map(|v| v.to_string());
    let tag = matches.value_of("tag").map(|v| v.to_string());

    let suppress_output = !matches.is_present("nocapture");

    Ok(CliOptions {
        feature,
        filter,
        tag,
        suppress_output,
    })
}
