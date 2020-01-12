// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::PathBuf;
use std::process;

use crate::cli::make_app;
use crate::globwalk::{glob, GlobWalkerBuilder};
use crate::{OutputVisitor, Scenario, Steps, World};

pub struct CucumberBuilder<W: World, O: OutputVisitor> {
    output: O,
    features: Vec<PathBuf>,
    setup: Option<fn() -> ()>,
    before: Vec<fn(&Scenario) -> ()>,
    after: Vec<fn(&Scenario) -> ()>,
    steps: Steps<W>,
    options: crate::cli::CliOptions,
}

impl<W: World, O: OutputVisitor> CucumberBuilder<W, O> {
    pub fn new(output: O) -> Self {
        CucumberBuilder {
            output,
            features: vec![],
            setup: None,
            before: vec![],
            after: vec![],
            steps: Steps::default(),
            options: crate::cli::CliOptions::default(),
        }
    }

    pub fn setup(&mut self, function: fn() -> ()) -> &mut Self {
        self.setup = Some(function);
        self
    }

    pub fn features(&mut self, features: Vec<PathBuf>) -> &mut Self {
        let mut features = features
            .iter()
            .map(|path| match path.canonicalize() {
                Ok(p) => GlobWalkerBuilder::new(p, "*.feature")
                    .case_insensitive(true)
                    .build()
                    .expect("feature path is invalid"),
                Err(e) => {
                    eprintln!("{}", e);
                    eprintln!("There was an error parsing {:?}; aborting.", path);
                    process::exit(1);
                }
            })
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path().to_owned())
            .collect::<Vec<_>>();
        features.sort();

        self.features = features;
        self
    }

    pub fn before(&mut self, functions: Vec<fn(&Scenario) -> ()>) -> &mut Self {
        self.before = functions;
        self
    }

    pub fn add_before(&mut self, function: fn(&Scenario) -> ()) -> &mut Self {
        self.before.push(function);
        self
    }

    pub fn after(&mut self, functions: Vec<fn(&Scenario) -> ()>) -> &mut Self {
        self.after = functions;
        self
    }

    pub fn add_after(&mut self, function: fn(&Scenario) -> ()) -> &mut Self {
        self.after.push(function);
        self
    }

    pub fn steps(&mut self, steps: Steps<W>) -> &mut Self {
        self.steps = steps;
        self
    }

    pub fn options(&mut self, options: crate::cli::CliOptions) -> &mut Self {
        self.options = options;
        self
    }

    pub fn run(mut self) -> bool {
        if let Some(feature) = self.options.feature.as_ref() {
            let features = glob(feature)
                .expect("feature glob is invalid")
                .filter_map(Result::ok)
                .map(|entry| entry.path().to_owned())
                .collect::<Vec<_>>();
            self.features(features);
        }

        if let Some(setup) = self.setup {
            setup();
        }

        self.steps.run(
            self.features,
            &self.before,
            &self.after,
            self.options,
            &mut self.output,
        )
    }

    pub fn command_line(mut self) -> bool {
        let options = make_app().unwrap();
        self.options(options);
        self.run()
    }
}
