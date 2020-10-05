// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use futures::StreamExt;

use crate::steps::Steps;
use crate::{EventHandler, World};
use std::time::Duration;

pub struct Cucumber<W: World> {
    steps: Steps<W>,
    features: Vec<gherkin::Feature>,
    event_handler: Box<dyn EventHandler>,
    step_timeout: Option<Duration>,
}

impl<W: World> Default for Cucumber<W> {
    fn default() -> Self {
        Cucumber {
            steps: Default::default(),
            features: Default::default(),
            event_handler: Box::new(crate::output::BasicOutput::default()),
            step_timeout: None,
        }
    }
}

impl<W: World> Cucumber<W> {
    pub fn new() -> Cucumber<W> {
        Default::default()
    }

    pub fn with_handler<O: EventHandler>(event_handler: O) -> Self {
        Cucumber {
            steps: Default::default(),
            features: Default::default(),
            event_handler: Box::new(event_handler),
            step_timeout: None,
        }
    }

    pub fn steps(mut self, steps: Steps<W>) -> Self {
        self.steps.append(steps);
        self
    }

    pub fn features(mut self, features: &[&str]) -> Self {
        let features = features
            .iter()
            .map(|path| match std::path::Path::new(path).canonicalize() {
                Ok(p) => globwalk::GlobWalkerBuilder::new(p, "*.feature")
                    .case_insensitive(true)
                    .build()
                    .expect("feature path is invalid"),
                Err(e) => {
                    eprintln!("{}", e);
                    eprintln!("There was an error parsing {:?}; aborting.", path);
                    std::process::exit(1);
                }
            })
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| gherkin::Feature::parse_path(entry.path()))
            .collect::<Result<Vec<_>, _>>();

        let mut features = features.unwrap_or_else(|e| panic!(e));
        features.sort();

        self.features = features;
        self
    }

    pub fn step_timeout(mut self, step_timeout: Duration) -> Self {
        self.step_timeout = Some(step_timeout);
        self
    }

    pub async fn run(mut self) {
        let runner = crate::runner::Runner::new(
            self.steps.steps,
            std::rc::Rc::new(self.features),
            self.step_timeout,
        );
        let mut stream = runner.run();

        while let Some(event) = stream.next().await {
            self.event_handler.handle_event(event);
        }
    }
}
