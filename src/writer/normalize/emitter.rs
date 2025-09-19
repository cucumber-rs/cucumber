// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Emitter trait for normalized event emission.

use std::future::Future;

use crate::Writer;

/// [`Queue`] which can remember its current item ([`Feature`], [`Rule`],
/// [`Scenario`] or [`Step`]) and pass events connected to it to the provided
/// [`Writer`].
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
/// [`Queue`]: super::queue::Queue
pub trait Emitter<World> {
    /// Currently outputted key and value from this [`Queue`].
    ///
    /// [`Queue`]: super::queue::Queue
    type Current;

    /// Currently outputted item ([`Feature`], [`Rule`], [`Scenario`] or
    /// [`Step`]). If returned from [`Self::emit()`], means that all events
    /// associated with that item were passed to the underlying [`Writer`], so
    /// should be removed from the [`Queue`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    /// [`Queue`]: super::queue::Queue
    type Emitted;

    /// Path to the [`Self::Emitted`] item. For [`Feature`] its `()`, as it's
    /// top-level item. For [`Scenario`] it's
    /// `(`[`Feature`]`, `[`Option`]`<`[`Rule`]`>)`, because [`Scenario`]
    /// definitely has parent [`Feature`] and optionally can have parent
    /// [`Rule`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    type EmittedPath;

    /// Currently outputted key and value from this [`Queue`].
    ///
    /// [`Queue`]: super::queue::Queue
    fn current_item(self) -> Option<Self::Current>;

    /// Passes events of the current item ([`Feature`], [`Rule`], [`Scenario`]
    /// or [`Step`]) to the provided [`Writer`].
    ///
    /// If this method returns [`Some`], then all events of the current item
    /// were passed to the provided [`Writer`] and that means it should be
    /// [`remove`]d.
    ///
    /// [`remove`]: super::queue::Queue::remove()
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    fn emit<W: Writer<World>>(
        self,
        path: Self::EmittedPath,
        writer: &mut W,
        cli: &W::Cli,
    ) -> impl Future<Output = Option<Self::Emitted>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, event::{Cucumber, Metadata}, parser, Writer};
    use crate::test_utils::common::{EmptyCli, TestWorld};

    // Using common TestWorld from test_utils

    // Mock Emitter for testing
    struct MockEmitter {
        should_emit: bool,
        events: Vec<String>,
    }

    impl MockEmitter {
        fn new(should_emit: bool) -> Self {
            Self {
                should_emit,
                events: Vec::new(),
            }
        }
    }

    impl Emitter<TestWorld> for MockEmitter {
        type Current = String;
        type Emitted = u32;
        type EmittedPath = ();

        fn current_item(self) -> Option<Self::Current> {
            if self.should_emit {
                Some("current_item".to_string())
            } else {
                None
            }
        }

        async fn emit<W: Writer<TestWorld>>(
            self,
            _path: Self::EmittedPath,
            _writer: &mut W,
            _cli: &W::Cli,
        ) -> Option<Self::Emitted> {
            if self.should_emit {
                Some(42)
            } else {
                None
            }
        }
    }

    // Mock Writer for testing
    struct MockWriter {
        events: Vec<String>,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                events: Vec::new(),
            }
        }
    }

    impl Writer<TestWorld> for MockWriter {
        type Cli = EmptyCli;

        async fn handle_event(
            &mut self,
            event: parser::Result<Event<crate::event::Cucumber<TestWorld>>>,
            _cli: &Self::Cli,
        ) {
            if let Ok(ev) = event {
                let event_name = match ev.value {
                    Cucumber::Started => "Started",
                    Cucumber::ParsingFinished { .. } => "ParsingFinished",
                    Cucumber::Finished => "Finished",
                    Cucumber::Feature(_, _) => "Feature",
                };
                self.events.push(event_name.to_string());
            }
        }
    }

    #[tokio::test]
    async fn test_emitter_with_current_item() {
        let emitter = MockEmitter::new(true);
        let mut writer = MockWriter::new();
        
        // Test that current_item returns Some when should_emit is true
        let current = emitter.current_item();
        assert_eq!(current, Some("current_item".to_string()));
        
        // Test emit returns Some when should_emit is true
        let emitter2 = MockEmitter::new(true);
        let result = emitter2.emit((), &mut writer, &()).await;
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_emitter_without_current_item() {
        let emitter = MockEmitter::new(false);
        let mut writer = MockWriter::new();
        
        // Test that current_item returns None when should_emit is false
        let current = emitter.current_item();
        assert_eq!(current, None);
        
        // Test emit returns None when should_emit is false
        let emitter2 = MockEmitter::new(false);
        let result = emitter2.emit((), &mut writer, &()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_emitter_with_writer() {
        let emitter = MockEmitter::new(true);
        let mut writer = MockWriter::new();
        
        // Test that emit can work with a writer
        let result = emitter.emit((), &mut writer, &()).await;
        assert!(result.is_some());
        
        // Writer should remain functional
        assert_eq!(writer.events.len(), 0); // No events sent in this test
    }

    #[test]
    fn test_emitter_type_definitions() {
        // Test that the type definitions are correct
        let emitter = MockEmitter::new(true);
        
        // These should compile, proving the associated types are correctly defined
        let _current: Option<String> = emitter.current_item();
    }

    #[test]
    fn test_emitter_path_types() {
        // Test that EmittedPath type works correctly
        fn test_path(_path: <MockEmitter as Emitter<TestWorld>>::EmittedPath) {}
        test_path(());
    }

    #[test]
    fn test_emitter_trait_bounds() {
        // Test that the trait bounds are satisfied
        fn requires_emitter<T: Emitter<TestWorld>>(_: T) {}
        let emitter = MockEmitter::new(true);
        requires_emitter(emitter);
    }
}