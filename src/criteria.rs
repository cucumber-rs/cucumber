use std::ops::{BitAnd, BitOr};

use gherkin::{Feature, Rule, Scenario};

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Pattern {
    Regex(regex::Regex),
    Literal(String),
}

impl From<String> for Pattern {
    fn from(x: String) -> Self {
        Pattern::Literal(x)
    }
}
impl From<&str> for Pattern {
    fn from(x: &str) -> Self {
        Pattern::Literal(x.to_string())
    }
}

impl From<regex::Regex> for Pattern {
    fn from(x: regex::Regex) -> Self {
        Pattern::Regex(x)
    }
}

impl Pattern {
    fn eval(&self, input: &str) -> bool {
        match self {
            Pattern::Regex(regex) => regex.is_match(input),
            Pattern::Literal(literal) => literal == input,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Criteria {
    And(Box<Criteria>, Box<Criteria>),
    Or(Box<Criteria>, Box<Criteria>),
    Scenario(Pattern),
    Rule(Pattern),
    Feature(Pattern),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Context {
    Scenario,
    Rule,
    Feature,
}

impl Context {
    pub fn is_scenario(&self) -> bool {
        match self {
            Context::Scenario => true,
            _ => false,
        }
    }

    pub fn is_rule(&self) -> bool {
        match self {
            Context::Rule => true,
            _ => false,
        }
    }

    pub fn is_feature(&self) -> bool {
        match self {
            Context::Feature => true,
            _ => false,
        }
    }
}

impl Criteria {
    pub(crate) fn context(&self) -> Context {
        match self {
            Criteria::And(a, b) => std::cmp::min(a.context(), b.context()),
            Criteria::Or(a, b) => std::cmp::min(a.context(), b.context()),
            Criteria::Scenario(_) => Context::Scenario,
            Criteria::Rule(_) => Context::Rule,
            Criteria::Feature(_) => Context::Feature,
        }
    }

    pub(crate) fn eval(
        &self,
        feature: &Feature,
        rule: Option<&Rule>,
        scenario: Option<&Scenario>,
    ) -> bool {
        match self {
            Criteria::And(a, b) => {
                a.eval(feature, rule, scenario) && b.eval(feature, rule, scenario)
            }
            Criteria::Or(a, b) => {
                a.eval(feature, rule, scenario) || b.eval(feature, rule, scenario)
            }
            Criteria::Scenario(pattern) if scenario.is_some() => {
                pattern.eval(&scenario.unwrap().name)
            }
            Criteria::Rule(pattern) if rule.is_some() => pattern.eval(&rule.unwrap().name),
            Criteria::Feature(pattern) => pattern.eval(&feature.name),
            _ => false,
        }
    }
}

impl BitAnd for Criteria {
    type Output = Criteria;

    fn bitand(self, rhs: Self) -> Self::Output {
        Criteria::And(Box::new(self), Box::new(rhs))
    }
}

impl BitOr for Criteria {
    type Output = Criteria;

    fn bitor(self, rhs: Self) -> Self::Output {
        Criteria::Or(Box::new(self), Box::new(rhs))
    }
}

pub fn scenario<P: Into<Pattern>>(pattern: P) -> Criteria {
    Criteria::Scenario(pattern.into())
}

pub fn rule<P: Into<Pattern>>(pattern: P) -> Criteria {
    Criteria::Rule(pattern.into())
}

pub fn feature<P: Into<Pattern>>(pattern: P) -> Criteria {
    Criteria::Feature(pattern.into())
}
