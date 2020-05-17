// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use futures::StreamExt;

use crate::World;
use crate::event::{CucumberEvent, ScenarioEvent};
use crate::steps::Steps;

fn scenario_event_handler(scenario: std::rc::Rc<gherkin::Scenario>, event: ScenarioEvent) {
    match event {
        ScenarioEvent::Starting => println!("Scenario '{}' starting", &scenario.name),
        ScenarioEvent::Background(step, event) => match event {
            crate::event::StepEvent::Unimplemented => println!("Background step '{}' unimplemented", step.to_string()),
            crate::event::StepEvent::Skipped => println!("Background step '{}' skipped", step.to_string()),
            crate::event::StepEvent::Passed(_) => println!("Background step '{}' passed", step.to_string()),
            crate::event::StepEvent::Failed(_, _) => println!("Background step '{}' failed", step.to_string()),
        }
        ScenarioEvent::Step(step, event) => match event {
            crate::event::StepEvent::Unimplemented => println!("Step '{}' unimplemented", step.to_string()),
            crate::event::StepEvent::Skipped => println!("Step '{}' skipped", step.to_string()),
            crate::event::StepEvent::Passed(_) => println!("Step '{}' passed", step.to_string()),
            crate::event::StepEvent::Failed(_, _) => println!("Step '{}' failed", step.to_string()),
        }
        ScenarioEvent::Skipped => println!("Scenario '{}' skipped", &scenario.name),
        ScenarioEvent::Passed => println!("Scenario '{}' passed", &scenario.name),
        ScenarioEvent::Failed => println!("Scenario '{}' failed", &scenario.name),
    }
}

fn event_handler(event: CucumberEvent) {
    match event {
        CucumberEvent::Starting => println!("Cucumber test runner starting"),
        CucumberEvent::Feature(feature, event) => match event {
            crate::event::FeatureEvent::Starting => println!("Feature '{}' starting", &feature.name),
            crate::event::FeatureEvent::Scenario(scenario, event) => scenario_event_handler(scenario, event),
            crate::event::FeatureEvent::Rule(rule, event) => match event {
                crate::event::RuleEvent::Starting => println!("Rule '{}' starting", &rule.name),
                crate::event::RuleEvent::Scenario(scenario, event) => scenario_event_handler(scenario, event),
                crate::event::RuleEvent::Skipped => println!("Rule '{}' skipped", &rule.name),
                crate::event::RuleEvent::Passed => println!("Rule '{}' passed", &rule.name),
                crate::event::RuleEvent::Failed => println!("Rule '{}' failed", &rule.name),
            }
            crate::event::FeatureEvent::Finished => println!("Feature '{}' finished", &feature.name),
        }
        CucumberEvent::Finished => println!("Cucumber test runner finished"),
    }
}

pub struct Cucumber<W: World> {
    steps: Steps<W>,
    features: Vec<gherkin::Feature>,
    event_handler: fn(CucumberEvent) -> ()
}

impl<W: World> Cucumber<W> {
    pub fn new() -> Cucumber<W> {
        Cucumber {
            steps: Default::default(),
            features: Default::default(),
            event_handler,
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

    pub async fn run(self) {
        let runner = crate::runner::Runner::new(
            self.steps.steps, 
            std::rc::Rc::new(self.features));
        let mut stream = runner.run();

        while let Some(event) = stream.next().await {
            (self.event_handler)(event);
        }
    }
}