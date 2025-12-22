// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Event handling implementation for the libtest writer.

use std::{fmt::Debug, io, iter, mem};

use either::Either;

use crate::{
    Event, World,
    event::{self, Retries},
    parser,
    writer::{
        basic::{coerce_error, trim_path},
        out::WriteStrExt as _,
    },
};

use super::{
    cli::Cli,
    json_events::{LibTestJsonEvent, SuiteEvent, SuiteResults, TestEvent},
    utils::LibtestUtils,
    writer::Libtest,
};

impl<W: Debug + World, Out: io::Write> Libtest<W, Out> {
    /// Handles the provided [`event::Cucumber`].
    ///
    /// Until [`ParsingFinished`] is received, all the events are stored inside
    /// [`Libtest::events`] and outputted only after that event is received.
    /// This is done, because [`libtest`][1]'s first event must contain number
    /// of executed test cases.
    ///
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    /// [`ParsingFinished`]: event::Cucumber::ParsingFinished
    pub(super) fn handle_cucumber_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Cli,
    ) {
        use event::{Cucumber, Metadata};

        let unite = |ev: Result<(Cucumber<W>, Metadata), _>| {
            ev.map(|(e, m)| m.insert(e))
        };

        match (event.map(Event::split), self.parsed_all) {
            (event @ Ok((Cucumber::ParsingFinished { .. }, _)), false) => {
                self.parsed_all = true;

                let all_events =
                    iter::once(unite(event)).chain(mem::take(&mut self.events));
                for ev in all_events {
                    self.output_event(ev, cli);
                }
            }
            (event, false) => self.events.push(unite(event)),
            (event, true) => self.output_event(unite(event), cli),
        }
    }

    /// Outputs the provided [`event::Cucumber`].
    fn output_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Cli,
    ) {
        for ev in self.expand_cucumber_event(event, cli) {
            self.output
                .write_line(serde_json::to_string(&ev).unwrap_or_else(|e| {
                    panic!("Failed to serialize `LibTestJsonEvent`: {e}")
                }))
                .unwrap_or_else(|e| panic!("Failed to write: {e}"));
        }
    }

    /// Converts the provided [`event::Cucumber`] into [`LibTestJsonEvent`]s.
    fn expand_cucumber_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::Cucumber;

        match event.map(Event::split) {
            Ok((Cucumber::Started, meta)) => {
                self.started_at = Some(meta.at);
                Vec::new()
            }
            Ok((Cucumber::ParsingFinished { steps, parser_errors, .. }, _)) => {
                vec![
                    SuiteEvent::Started { test_count: steps + parser_errors }
                        .into(),
                ]
            }
            Ok((Cucumber::Finished, meta)) => {
                let exec_time = self
                    .started_at
                    .and_then(|started| meta.at.duration_since(started).ok())
                    .as_ref()
                    .map(std::time::Duration::as_secs_f64);

                let failed =
                    self.failed + self.parsing_errors + self.hook_errors;
                let results = SuiteResults {
                    passed: self.passed,
                    failed,
                    ignored: self.ignored,
                    measured: 0,
                    filtered_out: 0,
                    exec_time,
                };
                let ev = if failed == 0 {
                    SuiteEvent::Ok { results }
                } else {
                    SuiteEvent::Failed { results }
                }
                .into();

                vec![ev]
            }
            Ok((Cucumber::Feature(feature, ev), meta)) => {
                self.expand_feature_event(&feature, ev, meta, cli)
            }
            Err(e) => self.handle_parsing_error(e),
        }
    }

    /// Handles parsing errors by converting them to test events.
    fn handle_parsing_error(&mut self, e: parser::Error) -> Vec<LibTestJsonEvent> {
        self.parsing_errors += 1;

        let path = match &e {
            parser::Error::Parsing(e) => match &**e {
                gherkin::ParseFileError::Parsing { path, .. }
                | gherkin::ParseFileError::Reading { path, .. } => {
                    Some(path)
                }
            },
            parser::Error::ExampleExpansion(e) => e.path.as_ref(),
        };
        let name = path.and_then(|p| p.to_str()).map_or_else(
            || self.parsing_errors.to_string(),
            |p| p.escape_default().to_string(),
        );
        let name = format!("Feature: Parsing {name}");

        vec![
            TestEvent::started(name.clone()).into(),
            TestEvent::failed(name, None)
                .with_stdout(e.to_string())
                .into(),
        ]
    }

    /// Converts the provided [`event::Feature`] into [`LibTestJsonEvent`]s.
    fn expand_feature_event(
        &mut self,
        feature: &gherkin::Feature,
        ev: event::Feature<W>,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::{Feature, Rule};

        match ev {
            Feature::Started
            | Feature::Finished
            | Feature::Rule(_, Rule::Started | Rule::Finished) => Vec::new(),
            Feature::Rule(rule, Rule::Scenario(scenario, ev)) => self
                .expand_scenario_event(
                    feature,
                    Some(&rule),
                    &scenario,
                    ev,
                    meta,
                    cli,
                ),
            Feature::Scenario(scenario, ev) => self
                .expand_scenario_event(feature, None, &scenario, ev, meta, cli),
        }
    }

    /// Converts the provided [`event::Scenario`] into [`LibTestJsonEvent`]s.
    fn expand_scenario_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: event::RetryableScenario<W>,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::Scenario;

        let retries = ev.retries;
        match ev.event {
            Scenario::Started | Scenario::Finished => Vec::new(),
            Scenario::Hook(ty, ev) => self.expand_hook_event(
                feature, rule, scenario, ty, ev, retries, meta, cli,
            ),
            Scenario::Background(step, ev) => self.expand_step_event(
                feature, rule, scenario, &step, ev, retries, true, meta, cli,
            ),
            Scenario::Step(step, ev) => self.expand_step_event(
                feature, rule, scenario, &step, ev, retries, false, meta, cli,
            ),
            // We do use `print!()` intentionally here to support `libtest`
            // output capturing properly, which can only capture output from
            // the standard library's `print!()` macro.
            // This is the same as `tracing_subscriber::fmt::TestWriter` does
            // (check its documentation for details).
            #[expect( // intentional
                clippy::print_stdout,
                reason = "supporting `libtest` output capturing properly"
            )]
            Scenario::Log(msg) => {
                print!("{msg}");
                vec![]
            }
        }
    }

    /// Converts the provided [`event::Hook`] into [`LibTestJsonEvent`]s.
    // TODO: Needs refactoring.
    #[expect(clippy::too_many_arguments, reason = "needs refactoring")]
    fn expand_hook_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        hook: event::HookType,
        ev: event::Hook<W>,
        retries: Option<Retries>,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        match ev {
            event::Hook::Started => {
                LibtestUtils::step_started_at(self, meta, cli);
                Vec::new()
            }
            event::Hook::Passed => Vec::new(),
            event::Hook::Failed(world, info) => {
                self.hook_errors += 1;

                let name = LibtestUtils::test_case_name(
                    self,
                    feature,
                    rule,
                    scenario,
                    Either::Left(hook),
                    retries,
                );

                vec![
                    TestEvent::started(name.clone()).into(),
                    TestEvent::failed(name, LibtestUtils::step_exec_time(self, meta, cli))
                        .with_stdout(format!(
                            "{}{}",
                            coerce_error(&info),
                            world
                                .map(|w| format!("\n{w:#?}"))
                                .unwrap_or_default(),
                        ))
                        .into(),
                ]
            }
        }
    }

    /// Converts the provided [`event::Step`] into [`LibTestJsonEvent`]s.
    // TODO: Needs refactoring.
    #[expect(clippy::too_many_arguments, reason = "needs refactoring")]
    fn expand_step_event(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        step: &gherkin::Step,
        ev: event::Step<W>,
        retries: Option<Retries>,
        is_background: bool,
        meta: event::Metadata,
        cli: &Cli,
    ) -> Vec<LibTestJsonEvent> {
        use event::Step;

        let name = LibtestUtils::test_case_name(
            self,
            feature,
            rule,
            scenario,
            Either::Right((step, is_background)),
            retries,
        );

        let ev = match ev {
            Step::Started => {
                LibtestUtils::step_started_at(self, meta, cli);
                TestEvent::started(name)
            }
            Step::Passed { location, .. } => {
                self.passed += 1;

                let event = TestEvent::ok(name, LibtestUtils::step_exec_time(self, meta, cli));
                if cli.show_output {
                    event.with_stdout(format!(
                        "{}:{}:{} (defined){}",
                        feature
                            .path
                            .as_ref()
                            .and_then(|p| p.to_str().map(trim_path))
                            .unwrap_or(&feature.name),
                        step.position.line,
                        step.position.col,
                        location.map(|l| format!(
                            "\n{}:{}:{} (matched)",
                            l.path, l.line, l.column,
                        ))
                        .unwrap_or_default()
                    ))
                } else {
                    event
                }
            }
            Step::Skipped => {
                self.ignored += 1;

                let event =
                    TestEvent::ignored(name, LibtestUtils::step_exec_time(self, meta, cli));
                if cli.show_output {
                    event.with_stdout(format!(
                        "{}:{}:{} (defined)",
                        feature
                            .path
                            .as_ref()
                            .and_then(|p| p.to_str().map(trim_path))
                            .unwrap_or(&feature.name),
                        step.position.line,
                        step.position.col,
                    ))
                } else {
                    event
                }
            }
            Step::Failed { location, world, error, .. } => {
                if retries.is_some_and(|r| {
                    r.left > 0 && !matches!(error, event::StepError::NotFound)
                }) {
                    self.retried += 1;
                } else {
                    self.failed += 1;
                }

                TestEvent::failed(name, LibtestUtils::step_exec_time(self, meta, cli))
                    .with_stdout(format!(
                        "{}:{}:{} (defined){}\n{error}{}",
                        feature
                            .path
                            .as_ref()
                            .and_then(|p| p.to_str().map(trim_path))
                            .unwrap_or(&feature.name),
                        step.position.line,
                        step.position.col,
                        location.map(|l| format!(
                            "\n{}:{}:{} (matched)",
                            l.path, l.line, l.column,
                        ))
                        .unwrap_or_default(),
                        world.map(|w| format!("\n{w:#?}")).unwrap_or_default(),
                    ))
            }
        };

        vec![ev.into()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[derive(Debug)]
    struct MockWorld;
    impl World for MockWorld {}

    mod cucumber_event_tests {
        use super::*;

        #[test]
        fn handle_cucumber_event_before_parsing_finished() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli::default();
            
            // Create a mock Started event
            let meta = event::Metadata::new(SystemTime::now());
            let event = Ok(meta.insert(event::Cucumber::Started));
            
            writer.handle_cucumber_event(event, &cli);
            
            // Events should be stored, not processed yet
            assert_eq!(writer.events.len(), 1);
            assert!(!writer.parsed_all);
        }

        #[test]
        fn handle_cucumber_event_started_sets_timestamp() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli::default();
            
            let start_time = SystemTime::now();
            let meta = event::Metadata::new(start_time);
            let event = Ok(meta.insert(event::Cucumber::Started));
            
            // Simulate parsing finished to trigger processing
            writer.parsed_all = true;
            writer.handle_cucumber_event(event, &cli);
            
            assert_eq!(writer.started_at, Some(start_time));
        }

        #[test]
        fn expand_cucumber_event_parsing_finished() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli::default();
            
            let meta = event::Metadata::new(SystemTime::now());
            let event = Ok((event::Cucumber::ParsingFinished {
                steps: 10,
                parser_errors: 2,
                features: 3,
            }, meta));
            
            let events = writer.expand_cucumber_event(event, &cli);
            
            assert_eq!(events.len(), 1);
            // Verify it's a suite started event with correct test count
            if let LibTestJsonEvent::Suite { event: SuiteEvent::Started { test_count } } = &events[0] {
                assert_eq!(*test_count, 12); // 10 steps + 2 parser errors
            } else {
                panic!("Expected SuiteEvent::Started");
            }
        }

        #[test]
        fn expand_cucumber_event_finished_success() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli::default();
            
            writer.passed = 5;
            writer.ignored = 2;
            writer.failed = 0;
            writer.parsing_errors = 0;
            writer.hook_errors = 0;
            
            let start_time = SystemTime::now();
            writer.started_at = Some(start_time);
            let finish_time = start_time + Duration::from_secs(1);
            
            let meta = event::Metadata::new(finish_time);
            let event = Ok((event::Cucumber::Finished, meta));
            
            let events = writer.expand_cucumber_event(event, &cli);
            
            assert_eq!(events.len(), 1);
            if let LibTestJsonEvent::Suite { event: SuiteEvent::Ok { results } } = &events[0] {
                assert_eq!(results.passed, 5);
                assert_eq!(results.failed, 0);
                assert_eq!(results.ignored, 2);
                assert!(results.exec_time.is_some());
            } else {
                panic!("Expected SuiteEvent::Ok");
            }
        }

        #[test]
        fn expand_cucumber_event_finished_failure() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli::default();
            
            writer.passed = 3;
            writer.failed = 2;
            writer.parsing_errors = 1;
            writer.hook_errors = 0;
            
            let meta = event::Metadata::new(SystemTime::now());
            let event = Ok((event::Cucumber::Finished, meta));
            
            let events = writer.expand_cucumber_event(event, &cli);
            
            assert_eq!(events.len(), 1);
            if let LibTestJsonEvent::Suite { event: SuiteEvent::Failed { results } } = &events[0] {
                assert_eq!(results.passed, 3);
                assert_eq!(results.failed, 3); // 2 failed + 1 parsing error
                assert!(results.exec_time.is_none()); // No start time set
            } else {
                panic!("Expected SuiteEvent::Failed");
            }
        }
    }

    mod parsing_error_tests {
        use super::*;

        #[test]
        fn handle_parsing_error_increments_counter() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            
            let error = parser::Error::ExampleExpansion(gherkin::ExampleExpansionError {
                path: None,
                position: gherkin::Position::new(1, 1),
                kind: gherkin::ExampleExpansionErrorKind::MismatchedPlaceholders {
                    expected: vec!["test".to_string()],
                    actual: vec!["other".to_string()],
                },
            });
            
            let events = writer.handle_parsing_error(error);
            
            assert_eq!(writer.parsing_errors, 1);
            assert_eq!(events.len(), 2); // Started + Failed events
        }

        #[test]
        fn handle_parsing_error_creates_test_events() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            
            let error = parser::Error::ExampleExpansion(gherkin::ExampleExpansionError {
                path: Some(std::path::PathBuf::from("test.feature")),
                position: gherkin::Position::new(1, 1),
                kind: gherkin::ExampleExpansionErrorKind::MismatchedPlaceholders {
                    expected: vec!["test".to_string()],
                    actual: vec!["other".to_string()],
                },
            });
            
            let events = writer.handle_parsing_error(error);
            
            assert_eq!(events.len(), 2);
            
            // Check first event is Started
            if let LibTestJsonEvent::Test { event: TestEvent::Started(_) } = &events[0] {
                // Good
            } else {
                panic!("Expected TestEvent::Started");
            }
            
            // Check second event is Failed with error message
            if let LibTestJsonEvent::Test { event: TestEvent::Failed(inner) } = &events[1] {
                assert!(inner.stdout.is_some());
            } else {
                panic!("Expected TestEvent::Failed");
            }
        }
    }

    mod event_flow_tests {
        use super::*;

        #[test]
        fn events_stored_before_parsing_finished() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli::default();
            
            // Add some events before parsing is finished
            let meta = event::Metadata::new(SystemTime::now());
            let event1 = Ok(meta.insert(event::Cucumber::Started));
            let event2 = Ok(meta.insert(event::Cucumber::Started));
            
            writer.handle_cucumber_event(event1, &cli);
            writer.handle_cucumber_event(event2, &cli);
            
            assert_eq!(writer.events.len(), 2);
            assert!(!writer.parsed_all);
            assert!(writer.output.is_empty()); // No output yet
        }

        #[test]
        fn events_flushed_on_parsing_finished() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli::default();
            
            // Add some events before parsing is finished
            let meta = event::Metadata::new(SystemTime::now());
            let started_event = Ok(meta.insert(event::Cucumber::Started));
            writer.handle_cucumber_event(started_event, &cli);
            
            assert_eq!(writer.events.len(), 1);
            
            // Now send parsing finished - this should flush all events
            let parsing_finished_event = Ok(meta.insert(event::Cucumber::ParsingFinished {
                steps: 5,
                parser_errors: 0,
                features: 1,
            }));
            writer.handle_cucumber_event(parsing_finished_event, &cli);
            
            assert_eq!(writer.events.len(), 0); // Events flushed
            assert!(writer.parsed_all);
            assert!(!writer.output.is_empty()); // Output generated
        }
    }
}