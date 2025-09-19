// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! CLI configuration and options for the libtest writer.

use std::str::FromStr;

/// CLI options of a [`Libtest`] [`Writer`].
///
/// [`Libtest`]: crate::writer::Libtest
/// [`Writer`]: crate::Writer
#[derive(Clone, Debug, Default, clap::Args)]
#[group(skip)]
pub struct Cli {
    /// Formatting of the output.
    #[arg(long, value_name = "json")]
    pub format: Option<Format>,

    /// Show captured stdout of successful tests. Currently, outputs only step
    /// function location.
    #[arg(long)]
    pub show_output: bool,

    /// Show execution time of each test.
    #[arg(long, value_name = "plain|colored", default_missing_value = "plain")]
    pub report_time: Option<ReportTime>,

    /// Enable nightly-only flags.
    #[arg(short = 'Z')]
    pub nightly: Option<String>,
}

/// Output formats.
///
/// Currently, supports only JSON.
#[derive(Clone, Copy, Debug)]
pub enum Format {
    /// [`libtest`][1]'s JSON format.
    ///
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    Json,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            s @ ("pretty" | "terse" | "junit") => {
                Err(format!("`{s}` option is not supported yet"))
            }
            s => Err(format!(
                "Unknown option `{s}`, expected `pretty` or `json`",
            )),
        }
    }
}

/// Format of reporting time.
#[derive(Clone, Copy, Debug)]
pub enum ReportTime {
    /// Plain time reporting.
    Plain,

    /// Colored time reporting.
    Colored,
}

impl FromStr for ReportTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(Self::Plain),
            "colored" => Ok(Self::Colored),
            s => Err(format!(
                "Unknown option `{s}`, expected `plain` or `colored`",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod format_tests {
        use super::*;

        #[test]
        fn format_from_str_json() {
            assert!(matches!(Format::from_str("json"), Ok(Format::Json)));
            assert!(matches!(Format::from_str("JSON"), Ok(Format::Json)));
            assert!(matches!(Format::from_str("Json"), Ok(Format::Json)));
        }

        #[test]
        fn format_from_str_unsupported() {
            let result = Format::from_str("pretty");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not supported yet"));

            let result = Format::from_str("terse");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not supported yet"));

            let result = Format::from_str("junit");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not supported yet"));
        }

        #[test]
        fn format_from_str_unknown() {
            let result = Format::from_str("unknown");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Unknown option"));
            assert!(result.unwrap_err().contains("expected `pretty` or `json`"));
        }

        #[test]
        fn format_debug() {
            let format = Format::Json;
            assert_eq!(format!("{format:?}"), "Json");
        }

        #[test]
        fn format_clone_copy() {
            let format1 = Format::Json;
            let format2 = format1; // Should work due to Copy trait
            assert!(matches!(format1, Format::Json));
            assert!(matches!(format2, Format::Json));
        }
    }

    mod report_time_tests {
        use super::*;

        #[test]
        fn report_time_from_str_plain() {
            assert!(matches!(ReportTime::from_str("plain"), Ok(ReportTime::Plain)));
            assert!(matches!(ReportTime::from_str("PLAIN"), Ok(ReportTime::Plain)));
            assert!(matches!(ReportTime::from_str("Plain"), Ok(ReportTime::Plain)));
        }

        #[test]
        fn report_time_from_str_colored() {
            assert!(matches!(ReportTime::from_str("colored"), Ok(ReportTime::Colored)));
            assert!(matches!(ReportTime::from_str("COLORED"), Ok(ReportTime::Colored)));
            assert!(matches!(ReportTime::from_str("Colored"), Ok(ReportTime::Colored)));
        }

        #[test]
        fn report_time_from_str_unknown() {
            let result = ReportTime::from_str("unknown");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Unknown option"));
            assert!(result.unwrap_err().contains("expected `plain` or `colored`"));
        }

        #[test]
        fn report_time_debug() {
            let plain = ReportTime::Plain;
            let colored = ReportTime::Colored;
            assert_eq!(format!("{plain:?}"), "Plain");
            assert_eq!(format!("{colored:?}"), "Colored");
        }

        #[test]
        fn report_time_clone_copy() {
            let time1 = ReportTime::Plain;
            let time2 = time1; // Should work due to Copy trait
            assert!(matches!(time1, ReportTime::Plain));
            assert!(matches!(time2, ReportTime::Plain));
        }
    }

    mod cli_tests {
        use super::*;

        #[test]
        fn cli_default() {
            let cli = Cli::default();
            assert!(cli.format.is_none());
            assert!(!cli.show_output);
            assert!(cli.report_time.is_none());
            assert!(cli.nightly.is_none());
        }

        #[test]
        fn cli_clone() {
            let cli1 = Cli {
                format: Some(Format::Json),
                show_output: true,
                report_time: Some(ReportTime::Colored),
                nightly: Some("unstable".to_string()),
            };
            let cli2 = cli1.clone();
            
            assert!(matches!(cli2.format, Some(Format::Json)));
            assert!(cli2.show_output);
            assert!(matches!(cli2.report_time, Some(ReportTime::Colored)));
            assert_eq!(cli2.nightly, Some("unstable".to_string()));
        }

        #[test]
        fn cli_debug() {
            let cli = Cli::default();
            let debug_str = format!("{cli:?}");
            assert!(debug_str.contains("Cli"));
        }
    }
}