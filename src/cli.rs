use clap::{Arg, App};
use regex::Regex;

#[derive(Debug)]
pub enum CliError {
    InvalidFilterRegex
}

pub struct CliOptions {
    pub filter: Option<Regex>
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
        .get_matches();

    let filter = if let Some(filter) = matches.value_of("filter") {
        let regex = Regex::new(filter).map_err(|_| CliError::InvalidFilterRegex)?;
        Some(regex)
    } else {
        None
    };

    Ok(CliOptions {
        filter: filter
    })
}
