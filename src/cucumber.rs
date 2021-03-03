// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::Path,
    rc::Rc,
};
use std::{pin::Pin, time::Duration};

use futures::{Future, StreamExt};
use gherkin::ParseFileError;
use regex::Regex;

use crate::{criteria::Criteria, steps::Steps};
use crate::{EventHandler, World};

pub(crate) type LifecycleFuture = Pin<Box<dyn Future<Output = ()>>>;

#[derive(Clone)]
pub struct LifecycleContext {
    pub(crate) context: Rc<Context>,
    pub feature: Rc<gherkin::Feature>,
    pub rule: Option<Rc<gherkin::Rule>>,
    pub scenario: Option<Rc<gherkin::Scenario>>,
}

impl LifecycleContext {
    #[inline]
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.context.get()
    }
}

pub type LifecycleFn = fn(LifecycleContext) -> LifecycleFuture;

pub struct Cucumber<W: World> {
    context: Context,

    steps: Steps<W>,
    features: Vec<gherkin::Feature>,
    event_handler: Box<dyn EventHandler>,

    /// If `Some`, enforce an upper bound on the amount
    /// of time a step is allowed to execute.
    /// If `Some`, also avoid indefinite locks during
    /// step clean-up handling (i.e. to recover panic info)
    step_timeout: Option<Duration>,

    /// If true, capture stdout and stderr content
    /// during tests.
    enable_capture: bool,

    /// If given, filters the scenario which are run
    scenario_filter: Option<Regex>,

    language: Option<String>,

    debug: bool,

    before: Vec<(Criteria, LifecycleFn)>,

    after: Vec<(Criteria, LifecycleFn)>,
}

pub struct StepContext {
    context: Rc<Context>,
    pub step: Rc<gherkin::Step>,
    pub matches: Vec<String>,
}

impl StepContext {
    #[inline]
    pub(crate) fn new(context: Rc<Context>, step: Rc<gherkin::Step>, matches: Vec<String>) -> Self {
        Self {
            context,
            step,
            matches,
        }
    }

    #[inline]
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.context.get()
    }
}

#[derive(Default)]
pub struct Context {
    data: HashMap<TypeId, Box<dyn Any>>,
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|x| x.downcast_ref::<T>())
    }

    pub fn insert<T: Any>(&mut self, value: T) {
        self.data.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn add<T: Any>(mut self, value: T) -> Self {
        self.insert(value);
        self
    }
}

impl<W: World> Default for Cucumber<W> {
    fn default() -> Self {
        Cucumber {
            context: Default::default(),
            steps: Default::default(),
            features: Default::default(),
            event_handler: Box::new(crate::output::BasicOutput::new(false)),
            step_timeout: None,
            enable_capture: true,
            debug: false,
            scenario_filter: None,
            language: None,
            before: vec![],
            after: vec![],
        }
    }
}

impl<W: World> Cucumber<W> {
    /// Construct a default `Cucumber` instance.
    ///
    /// Comes with the default `EventHandler` implementation responsible for
    /// printing test execution progress.
    pub fn new() -> Cucumber<W> {
        Default::default()
    }

    /// Construct a `Cucumber` instance with a custom `EventHandler`.
    pub fn with_handler<O: EventHandler>(event_handler: O) -> Self {
        Cucumber {
            context: Default::default(),
            steps: Default::default(),
            features: Default::default(),
            event_handler: Box::new(event_handler),
            step_timeout: None,
            enable_capture: true,
            debug: false,
            scenario_filter: None,
            language: None,
            before: vec![],
            after: vec![],
        }
    }

    /// Add some steps to the Cucumber instance.
    ///
    /// Does *not* replace any previously added steps.
    pub fn steps(mut self, steps: Steps<W>) -> Self {
        self.steps.append(steps);
        self
    }

    /// A collection of directory paths that will be walked to
    /// find ".feature" files.
    ///
    /// Removes any previously-supplied features.
    pub fn features<P: AsRef<Path>>(mut self, features: impl IntoIterator<Item = P>) -> Self {
        let features = features
            .into_iter()
            .map(|path| match path.as_ref().canonicalize() {
                Ok(p) if p.ends_with(".feature") => {
                    let env = match self.language.as_ref() {
                        Some(lang) => gherkin::GherkinEnv::new(lang).unwrap(),
                        None => Default::default(),
                    };
                    vec![gherkin::Feature::parse_path(&p, env)]
                }
                Ok(p) => {
                    let walker = globwalk::GlobWalkerBuilder::new(p, "*.feature")
                        .case_insensitive(true)
                        .build()
                        .expect("feature path is invalid");
                    walker
                        .filter_map(Result::ok)
                        .map(|entry| {
                            let env = match self.language.as_ref() {
                                Some(lang) => gherkin::GherkinEnv::new(lang).unwrap(),
                                None => Default::default(),
                            };
                            gherkin::Feature::parse_path(entry.path(), env)
                        })
                        .collect::<Vec<_>>()
                }
                Err(e) => {
                    eprintln!("{}", e);
                    eprintln!("There was an error parsing {:?}; aborting.", path.as_ref());
                    std::process::exit(1);
                }
            })
            .flatten()
            .collect::<Result<Vec<_>, _>>();

        let mut features = features.unwrap_or_else(|e| match e {
            ParseFileError::Reading { path, source } => {
                eprintln!("Error reading '{}':", path.display());
                eprintln!("{:?}", source);
                std::process::exit(1);
            }
            ParseFileError::Parsing {
                path,
                error,
                source,
            } => {
                eprintln!("Error parsing '{}':", path.display());
                if let Some(error) = error {
                    eprintln!("{}", error);
                }
                eprintln!("{:?}", source);
                std::process::exit(1);
            }
        });
        features.sort();

        self.features = features;
        self
    }

    /// If `Some`, enforce an upper bound on the amount
    /// of time a step is allowed to execute.
    /// If `Some`, also avoid indefinite locks during
    /// step clean-up handling (i.e. to recover panic info)
    pub fn step_timeout(mut self, step_timeout: Duration) -> Self {
        self.step_timeout = Some(step_timeout);
        self
    }

    /// If true, capture stdout and stderr content
    /// during tests.
    pub fn enable_capture(mut self, enable_capture: bool) -> Self {
        self.enable_capture = enable_capture;
        self
    }

    pub fn scenario_regex(mut self, regex: &str) -> Self {
        let regex = Regex::new(regex).expect("Error compiling scenario regex");
        self.scenario_filter = Some(regex);
        self
    }

    /// Call this to incorporate command line options into the configuration.
    pub fn cli(self) -> Self {
        let opts = crate::cli::make_app();
        let mut s = self;

        if let Some(re) = opts.scenario_filter {
            s = s.scenario_regex(&re);
        }

        if opts.nocapture {
            s = s.enable_capture(false);
        }

        if opts.debug {
            s = s.debug(true);
        }

        s
    }

    /// Set the default language to assume for each .feature file.
    pub fn language(mut self, language: &str) -> Self {
        if gherkin::is_language_supported(language) {
            self.language = Some(language.to_string());
        } else {
            eprintln!(
                "ERROR: Provided language '{}' not supported; ignoring.",
                language
            );
        }

        self
    }

    pub fn before(mut self, criteria: Criteria, handler: LifecycleFn) -> Self {
        self.before.push((criteria, handler));
        self
    }

    pub fn after(mut self, criteria: Criteria, handler: LifecycleFn) -> Self {
        self.after.push((criteria, handler));
        self
    }

    /// Enable printing stdout and stderr for every step, regardless of error state.
    pub fn debug(mut self, value: bool) -> Self {
        self.event_handler = Box::new(crate::output::BasicOutput::new(value));
        self.debug = value;
        self
    }

    pub fn context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }

    /// Run and report number of errors if any
    pub async fn run(mut self) -> crate::runner::RunResult {
        let runner = crate::runner::Runner::new(
            Rc::new(self.context),
            self.steps.steps,
            Rc::new(self.features),
            self.step_timeout,
            self.enable_capture,
            self.scenario_filter,
            self.before,
            self.after,
        );
        let mut stream = runner.run();

        while let Some(event) = stream.next().await {
            self.event_handler.handle_event(&event);

            if let crate::event::CucumberEvent::Finished(result) = event {
                return result;
            }
        }

        unreachable!("CucumberEvent::Finished must be fired")
    }

    /// Convenience function to run all tests and exit with error code 1 on failure.
    pub async fn run_and_exit(self) {
        let code = if self.run().await.failed() { 1 } else { 0 };
        std::process::exit(code);
    }
}
