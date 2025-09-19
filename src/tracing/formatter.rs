//! Event and field formatters for tracing integration with scenario markers.

use std::fmt;
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    field::RecordFields,
    fmt::{FmtContext, FormatEvent, FormatFields, format},
    registry::LookupSpan,
};

use crate::runner::basic::ScenarioId;
use super::visitor::IsScenarioIdSpan;

/// [`FormatFields`] wrapper skipping [`Span`]s with a [`ScenarioId`].
///
/// [`Span`]: tracing::Span
#[derive(Debug)]
pub struct SkipScenarioIdSpan<F>(pub F);

impl<'w, F: FormatFields<'w>> FormatFields<'w> for SkipScenarioIdSpan<F> {
    fn format_fields<R: RecordFields>(
        &self,
        writer: format::Writer<'w>,
        fields: R,
    ) -> fmt::Result {
        let mut is_scenario_span = IsScenarioIdSpan::new();
        fields.record(&mut is_scenario_span);
        if !is_scenario_span.is_scenario_span() {
            self.0.format_fields(writer, fields)?;
        }
        Ok(())
    }
}

/// [`FormatEvent`] wrapper, appending [`tracing::Event`]s with some markers,
/// to parse them later and retrieve optional [`ScenarioId`].
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Debug)]
pub struct AppendScenarioMsg<F>(pub F);

impl<S, N, F> FormatEvent<S, N> for AppendScenarioMsg<F>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        self.0.format_event(ctx, writer.by_ref(), event)?;

        if let Some(scenario_id) = ctx.event_scope().and_then(|scope| {
            scope
                .from_root()
                .find_map(|span| span.extensions().get::<ScenarioId>().copied())
        }) {
            writer.write_fmt(format_args!(
                "{}{scenario_id}",
                suffix::BEFORE_SCENARIO_ID,
            ))?;
        } else {
            writer.write_fmt(format_args!("{}", suffix::NO_SCENARIO_ID))?;
        }
        writer.write_fmt(format_args!("{}", suffix::END))
    }
}

/// String suffixes for parsing tracing events.
pub mod suffix {
    //! [`str`]ings appending [`tracing::Event`]s to separate them later.
    //!
    //! Every [`tracing::Event`] ends with:
    //!
    //! ([`BEFORE_SCENARIO_ID`][`ScenarioId`][`END`]|[`NO_SCENARIO_ID`][`END`])
    //!
    //! [`ScenarioId`]: crate::runner::basic::ScenarioId

    /// End of a [`tracing::Event`] message.
    pub const END: &str = "__cucumber__scenario";

    /// Separator before a [`ScenarioId`].
    ///
    /// [`ScenarioId`]: crate::runner::basic::ScenarioId
    pub const BEFORE_SCENARIO_ID: &str = "__";

    /// Separator in case there is no [`ScenarioId`].
    ///
    /// [`ScenarioId`]: crate::runner::basic::ScenarioId
    pub const NO_SCENARIO_ID: &str = "__unknown";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tracing_subscriber::fmt::format::{DefaultFields, Format};
    use tracing_subscriber::registry::Registry;

    struct TestWriter {
        buffer: Vec<u8>,
    }

    impl TestWriter {
        fn new() -> Self {
            Self { buffer: Vec::new() }
        }

        fn to_string(&self) -> String {
            String::from_utf8_lossy(&self.buffer).to_string()
        }
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl fmt::Write for TestWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.buffer.extend_from_slice(s.as_bytes());
            Ok(())
        }
    }

    #[test]
    fn test_suffix_constants() {
        assert_eq!(suffix::END, "__cucumber__scenario");
        assert_eq!(suffix::BEFORE_SCENARIO_ID, "__");
        assert_eq!(suffix::NO_SCENARIO_ID, "__unknown");
    }

    #[test]
    fn test_skip_scenario_id_span_creation() {
        let inner_formatter = DefaultFields::new();
        let formatter = SkipScenarioIdSpan(inner_formatter);
        
        // Test that wrapper was created successfully
        assert!(std::mem::size_of_val(&formatter) > 0);
    }

    #[test]
    fn test_append_scenario_msg_creation() {
        let inner_formatter = Format::default();
        let formatter = AppendScenarioMsg(inner_formatter);
        
        // Test that wrapper was created successfully
        assert!(std::mem::size_of_val(&formatter) > 0);
    }

    #[test]
    fn test_skip_scenario_id_span_with_normal_fields() {
        let inner_formatter = DefaultFields::new();
        let formatter = SkipScenarioIdSpan(inner_formatter);
        
        let mut writer = TestWriter::new();
        let fmt_writer = format::Writer::new(&mut writer);
        
        // Create a simple field set without scenario ID
        let fieldset = tracing::field::FieldSet::new(
            &["message"],
            tracing::callsite::Identifier::new(()),
        );
        let values = fieldset.value_set(&[(&fieldset.field("message").unwrap(), Some("test"))]);
        
        // This should format the fields normally
        let result = formatter.format_fields(fmt_writer, &values);
        assert!(result.is_ok());
    }

    #[test]
    fn test_skip_scenario_id_span_with_scenario_field() {
        let inner_formatter = DefaultFields::new();
        let formatter = SkipScenarioIdSpan(inner_formatter);
        
        let mut writer = TestWriter::new();
        let fmt_writer = format::Writer::new(&mut writer);
        
        // Create field set with scenario ID field
        let fieldset = tracing::field::FieldSet::new(
            &[ScenarioId::SPAN_FIELD_NAME],
            tracing::callsite::Identifier::new(()),
        );
        let values = fieldset.value_set(&[(&fieldset.field(ScenarioId::SPAN_FIELD_NAME).unwrap(), Some(&42u64))]);
        
        // This should skip formatting since it contains scenario ID
        let result = formatter.format_fields(fmt_writer, &values);
        assert!(result.is_ok());
        assert_eq!(writer.to_string(), ""); // Should be empty
    }

    #[test]
    fn test_append_scenario_msg_with_no_scenario() {
        let inner_formatter = Format::default();
        let formatter = AppendScenarioMsg(inner_formatter);
        
        let subscriber = Registry::default();
        let ctx = FmtContext::new(&subscriber, &DefaultFields::new());
        
        let mut writer = TestWriter::new();
        let fmt_writer = format::Writer::new(&mut writer);
        
        // Create a test event
        let metadata = tracing::Metadata::new(
            "test_event",
            "test_target",
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier::new(())),
            tracing::metadata::Kind::EVENT,
        );
        let values = metadata.fields().value_set(&[]);
        let event = Event::new(&metadata, &values);
        
        let result = formatter.format_event(&ctx, fmt_writer, &event);
        assert!(result.is_ok());
        
        let output = writer.to_string();
        assert!(output.contains(suffix::NO_SCENARIO_ID));
        assert!(output.contains(suffix::END));
    }

    #[test]
    fn test_format_event_error_handling() {
        struct FailingFormatter;
        
        impl<S, N> FormatEvent<S, N> for FailingFormatter
        where
            S: Subscriber + for<'a> LookupSpan<'a>,
            N: for<'a> FormatFields<'a> + 'static,
        {
            fn format_event(
                &self,
                _ctx: &FmtContext<'_, S, N>,
                _writer: format::Writer<'_>,
                _event: &Event<'_>,
            ) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        
        let formatter = AppendScenarioMsg(FailingFormatter);
        let subscriber = Registry::default();
        let ctx = FmtContext::new(&subscriber, &DefaultFields::new());
        
        let mut writer = TestWriter::new();
        let fmt_writer = format::Writer::new(&mut writer);
        
        let metadata = tracing::Metadata::new(
            "test_event",
            "test_target",
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier::new(())),
            tracing::metadata::Kind::EVENT,
        );
        let values = metadata.fields().value_set(&[]);
        let event = Event::new(&metadata, &values);
        
        let result = formatter.format_event(&ctx, fmt_writer, &event);
        assert!(result.is_err());
    }

    #[test]
    fn test_suffix_uniqueness() {
        // Ensure all suffixes are unique to avoid parsing conflicts
        let suffixes = vec![
            suffix::END,
            suffix::BEFORE_SCENARIO_ID,
            suffix::NO_SCENARIO_ID,
        ];
        
        for i in 0..suffixes.len() {
            for j in (i + 1)..suffixes.len() {
                assert_ne!(suffixes[i], suffixes[j], "Suffixes must be unique");
            }
        }
    }

    #[test]
    fn test_suffix_parsing_patterns() {
        // Test the expected patterns
        let with_scenario = format!("test message{}{}{}", suffix::BEFORE_SCENARIO_ID, "42", suffix::END);
        let without_scenario = format!("test message{}{}", suffix::NO_SCENARIO_ID, suffix::END);
        
        assert!(with_scenario.ends_with(suffix::END));
        assert!(without_scenario.ends_with(suffix::END));
        assert!(with_scenario.contains(suffix::BEFORE_SCENARIO_ID));
        assert!(without_scenario.contains(suffix::NO_SCENARIO_ID));
    }
}