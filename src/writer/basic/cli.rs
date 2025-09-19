//! CLI options and coloring configuration for Basic writer.

use std::str::FromStr;

use smart_default::SmartDefault;

use crate::cli::Colored;

/// CLI options of a [`Basic`] [`Writer`].
///
/// [`Basic`]: super::Basic
/// [`Writer`]: crate::Writer
#[derive(Clone, Copy, Debug, SmartDefault, clap::Args)]
#[group(skip)]
pub struct Cli {
    /// Verbosity of an output.
    ///
    /// `-v` is default verbosity, `-vv` additionally outputs world on failed
    /// steps, `-vvv` additionally outputs step's doc string (if present).
    #[arg(short, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Coloring policy for a console output.
    #[arg(
        long,
        value_name = "auto|always|never",
        default_value = "auto",
        global = true
    )]
    #[default(Coloring::Auto)]
    pub color: Coloring,
}

impl Colored for Cli {
    fn coloring(&self) -> Coloring {
        self.color
    }
}

/// Possible policies of a [`console`] output coloring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Coloring {
    /// Letting [`console::colors_enabled()`] to decide, whether output should
    /// be colored.
    Auto,

    /// Forcing of a colored output.
    Always,

    /// Forcing of a non-colored output.
    Never,
}

impl FromStr for Coloring {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err("possible options: auto, always, never"),
        }
    }
}

