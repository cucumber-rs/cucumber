//! Error handling utilities for JUnit XML writer.

use junit_report::{Duration, Report, TestCase, TestSuiteBuilder};

use crate::{parser, writer::basic::trim_path};

/// Handles parser and expansion errors by converting them to JUnit test suites.
pub struct ErrorHandler;

impl ErrorHandler {
    /// Handles the given [`parser::Error`] by adding it as a test suite to the report.
    pub fn handle_error(report: &mut Report, err: &parser::Error) {
        let (name, ty) = Self::extract_error_info(err);

        report.add_testsuite(
            TestSuiteBuilder::new("Errors")
                .add_testcase(TestCase::failure(
                    &name,
                    Duration::ZERO,
                    ty,
                    &err.to_string(),
                ))
                .build(),
        );
    }

    /// Extracts error information from a [`parser::Error`].
    ///
    /// Returns a tuple of (test_name, error_type).
    fn extract_error_info(err: &parser::Error) -> (String, &'static str) {
        match err {
            parser::Error::Parsing(err) => {
                let path = match err.as_ref() {
                    gherkin::ParseFileError::Reading { path, .. }
                    | gherkin::ParseFileError::Parsing { path, .. } => path,
                };
                (
                    format!(
                        "Feature{}",
                        path.to_str()
                            .map(|p| format!(": {}", trim_path(p)))
                            .unwrap_or_default(),
                    ),
                    "Parser Error",
                )
            }
            parser::Error::ExampleExpansion(err) => (
                format!(
                    "Feature: {}{}:{}",
                    err.path
                        .as_deref()
                        .and_then(|p| p.to_str().map(trim_path))
                        .map(|p| format!("{p}:"))
                        .unwrap_or_default(),
                    err.pos.line,
                    err.pos.col,
                ),
                "Example Expansion Error",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use junit_report::Report;

    use super::*;

    #[test]
    fn handles_parsing_error_with_path() {
        let mut report = Report::new();
        let path = PathBuf::from("/test/feature.feature");
        let parse_error = gherkin::ParseFileError::Reading {
            path: path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        };
        let parser_error = parser::Error::Parsing(Box::new(parse_error));

        ErrorHandler::handle_error(&mut report, &parser_error);

        assert_eq!(report.testsuites().len(), 1);
        let suite = &report.testsuites()[0];
        assert_eq!(suite.name(), "Errors");
        assert_eq!(suite.testcases().len(), 1);

        let testcase = &suite.testcases()[0];
        assert!(testcase.name().contains("Feature: feature.feature"));
        assert_eq!(testcase.result().as_failure().unwrap().type_(), "Parser Error");
    }

    #[test]
    fn handles_parsing_error_without_extension() {
        let mut report = Report::new();
        let path = PathBuf::from("/very/long/path/to/test/feature");
        let parse_error = gherkin::ParseFileError::Reading {
            path: path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied"),
        };
        let parser_error = parser::Error::Parsing(Box::new(parse_error));

        ErrorHandler::handle_error(&mut report, &parser_error);

        let suite = &report.testsuites()[0];
        let testcase = &suite.testcases()[0];
        // Path should be trimmed by trim_path function
        assert!(testcase.name().contains("Feature: test/feature"));
    }

    #[test]
    fn handles_example_expansion_error() {
        let mut report = Report::new();
        let expansion_error = gherkin::ExampleExpansionError {
            path: Some(PathBuf::from("/test/examples.feature")),
            pos: gherkin::LineCol { line: 10, col: 5 },
            source: gherkin::ExampleExpansionErrorSource::OutlineWithoutExamples,
        };
        let parser_error = parser::Error::ExampleExpansion(expansion_error);

        ErrorHandler::handle_error(&mut report, &parser_error);

        assert_eq!(report.testsuites().len(), 1);
        let suite = &report.testsuites()[0];
        assert_eq!(suite.name(), "Errors");
        assert_eq!(suite.testcases().len(), 1);

        let testcase = &suite.testcases()[0];
        assert_eq!(testcase.name(), "Feature: examples.feature:10:5");
        assert_eq!(testcase.result().as_failure().unwrap().type_(), "Example Expansion Error");
    }

    #[test]
    fn handles_example_expansion_error_without_path() {
        let mut report = Report::new();
        let expansion_error = gherkin::ExampleExpansionError {
            path: None,
            pos: gherkin::LineCol { line: 5, col: 1 },
            source: gherkin::ExampleExpansionErrorSource::OutlineWithoutExamples,
        };
        let parser_error = parser::Error::ExampleExpansion(expansion_error);

        ErrorHandler::handle_error(&mut report, &parser_error);

        let testcase = &report.testsuites()[0].testcases()[0];
        assert_eq!(testcase.name(), "Feature: 5:1");
    }

    #[test]
    fn extract_error_info_parsing_error() {
        let path = PathBuf::from("/test/scenario.feature");
        let parse_error = gherkin::ParseFileError::Parsing {
            path: path.clone(),
            source: gherkin::ParseError::new(
                gherkin::LineCol { line: 1, col: 1 },
                gherkin::ParseErrorType::MissingFeatureKeyword,
            ),
        };
        let parser_error = parser::Error::Parsing(Box::new(parse_error));

        let (name, error_type) = ErrorHandler::extract_error_info(&parser_error);

        assert_eq!(name, "Feature: scenario.feature");
        assert_eq!(error_type, "Parser Error");
    }

    #[test]
    fn extract_error_info_example_expansion_error() {
        let expansion_error = gherkin::ExampleExpansionError {
            path: Some(PathBuf::from("/test/outline.feature")),
            pos: gherkin::LineCol { line: 15, col: 10 },
            source: gherkin::ExampleExpansionErrorSource::OutlineWithoutExamples,
        };
        let parser_error = parser::Error::ExampleExpansion(expansion_error);

        let (name, error_type) = ErrorHandler::extract_error_info(&parser_error);

        assert_eq!(name, "Feature: outline.feature:15:10");
        assert_eq!(error_type, "Example Expansion Error");
    }
}