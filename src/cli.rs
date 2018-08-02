use clap::{Arg, App};
use regex::Regex;

#[derive(Debug)]
pub enum CliError {
    InvalidFilterRegex
}

pub struct CliOptions {
    pub filter: Option<Regex>,
    pub suppress_output: bool,
}

pub fn make_app<'a, 'b>() -> Result<CliOptions, CliError> {
    let matches = App::new("cucumber")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Brendan Molloy <brendan@bbqsrc.net>")
        .about("Run the tests, pet a dog!")
        .arg(Arg::with_name("filter")
            .short("f")
            .long("filter")
            .value_name("regex")
            .help("Regex to select scenarios from")
            .takes_value(true))
        .arg(Arg::with_name("nocapture")
            .long("nocapture")
            .help("Use this flag to disable suppression of output from tests"))
        .get_matches();

    let filter = if let Some(filter) = matches.value_of("filter") {
        let regex = Regex::new(filter).map_err(|_| CliError::InvalidFilterRegex)?;
        Some(regex)
    } else {
        None
    };

    let suppress_output = cfg!(feature = "nightly") && !matches.is_present("nocapture");

    Ok(CliOptions {
        filter: filter,
        suppress_output: suppress_output,
    })
}
