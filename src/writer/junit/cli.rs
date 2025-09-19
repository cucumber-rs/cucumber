//! CLI configuration for JUnit XML writer.

use crate::writer::Verbosity;

/// CLI options of a [`JUnit`] [`Writer`].
///
/// [`JUnit`]: super::JUnit
/// [`Writer`]: crate::Writer
#[derive(Clone, Copy, Debug, Default, clap::Args)]
#[group(skip)]
pub struct Cli {
    /// Verbosity of JUnit XML report output.
    ///
    /// `0` is default verbosity, `1` additionally outputs world on failed
    /// steps.
    #[arg(id = "junit-v", long = "junit-v", value_name = "0|1", global = true)]
    pub verbose: Option<u8>,
}

impl Cli {
    /// Converts CLI verbosity setting to [`Verbosity`] enum.
    #[must_use]
    pub const fn to_verbosity(self) -> Option<Verbosity> {
        match self.verbose {
            None => None,
            Some(0) => Some(Verbosity::Default),
            Some(_) => Some(Verbosity::ShowWorld),
        }
    }

    /// Creates a new [`Cli`] with the specified verbosity level.
    #[must_use]
    pub const fn with_verbosity(verbose: Option<u8>) -> Self {
        Self { verbose }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_default_has_no_verbosity() {
        let cli = Cli::default();
        assert_eq!(cli.verbose, None);
        assert_eq!(cli.to_verbosity(), None);
    }

    #[test]
    fn cli_verbosity_zero_maps_to_default() {
        let cli = Cli::with_verbosity(Some(0));
        assert_eq!(cli.verbose, Some(0));
        assert_eq!(cli.to_verbosity(), Some(Verbosity::Default));
    }

    #[test]
    fn cli_verbosity_one_maps_to_show_world() {
        let cli = Cli::with_verbosity(Some(1));
        assert_eq!(cli.verbose, Some(1));
        assert_eq!(cli.to_verbosity(), Some(Verbosity::ShowWorld));
    }

    #[test]
    fn cli_verbosity_high_value_maps_to_show_world() {
        let cli = Cli::with_verbosity(Some(5));
        assert_eq!(cli.verbose, Some(5));
        assert_eq!(cli.to_verbosity(), Some(Verbosity::ShowWorld));
    }

    #[test]
    fn cli_clone_preserves_verbosity() {
        let original = Cli::with_verbosity(Some(1));
        let cloned = original.clone();
        assert_eq!(original.verbose, cloned.verbose);
    }

    #[test]
    fn cli_debug_format_includes_verbose_field() {
        let cli = Cli::with_verbosity(Some(1));
        let debug_output = format!("{:?}", cli);
        assert!(debug_output.contains("verbose"));
        assert!(debug_output.contains("Some(1)"));
    }
}