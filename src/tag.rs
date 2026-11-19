// Copyright (c) 2018-2026  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Extension of a [`TagOperation`].

use gherkin::tagexpr::TagOperation;
use sealed::sealed;

/// Extension of a [`TagOperation`] allowing to evaluate it.
#[sealed]
pub trait Ext {
    /// Evaluates this [`TagOperation`] for the given `tags`.
    #[must_use]
    fn eval<I, S>(&self, tags: I) -> bool
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S> + Clone;
}

#[sealed]
impl Ext for TagOperation {
    fn eval<I, S>(&self, tags: I) -> bool
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S> + Clone,
    {
        match self {
            Self::And(l, r) => l.eval(tags.clone()) & r.eval(tags),
            Self::Or(l, r) => l.eval(tags.clone()) | r.eval(tags),
            Self::Not(t) => !t.eval(tags),
            Self::Tag(t) => tags.into_iter().any(|tag| tag.as_ref() == t),
        }
    }
}

/// Explicit tag-expression syntax pinned to `@cucumber/tag-expressions` 11.0.1.
///
/// Unlike legacy [`TagOperation`], this accepts empty expressions (true), bare
/// literals, and JavaScript whitespace/escaping. [`Self::evaluate`] consumes
/// complete tag strings such as `@smoke`. [`Self::matches`] adapts the native
/// AST's marker-free tags and includes inherited feature, Rule and Examples
/// tags.
/// Use it with `Cucumber::filter_run` or `runner::Hooks::before_matching`.
#[derive(Clone, Debug)]
pub struct Expression(Node);

#[derive(Clone, Debug)]
enum Node {
    True,
    Literal(String),
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
}

/// Invalid explicit tag-expression syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionError(String);

impl std::fmt::Display for ExpressionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for ExpressionError {}

impl std::str::FromStr for Expression {
    type Err = ExpressionError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let tokens = tokenize(input)?;
        if tokens.is_empty() {
            return Ok(Self(Node::True));
        }
        let mut parser = ExpressionParser { tokens, next: 0 };
        let expression = parser.or()?;
        if parser.next != parser.tokens.len() {
            return Err(ExpressionError(
                "expected an operator or a matching parenthesis".into(),
            ));
        }
        Ok(Self(expression))
    }
}

impl Expression {
    /// Evaluates the expression against complete, marker-preserving literals.
    #[must_use]
    pub fn evaluate<I, S>(&self, tags: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let tags = tags
            .into_iter()
            .map(|tag| tag.as_ref().to_owned())
            .collect::<Vec<_>>();
        self.0.evaluate(&tags)
    }

    /// Selects a native scenario using its own and inherited tags.
    ///
    /// Basic parser expansion already incorporates each selected Examples
    /// block's
    /// tags into the scenario. This method adds feature and Rule inheritance.
    #[must_use]
    pub fn matches(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
    ) -> bool {
        self.evaluate(
            feature
                .tags
                .iter()
                .chain(rule.into_iter().flat_map(|rule| &rule.tags))
                .chain(&scenario.tags)
                .map(|tag| format!("@{tag}")),
        )
    }
}

impl Node {
    fn evaluate(&self, tags: &[String]) -> bool {
        match self {
            Self::True => true,
            Self::Literal(value) => tags.contains(value),
            Self::Not(inner) => !inner.evaluate(tags),
            Self::And(left, right) => {
                left.evaluate(tags) && right.evaluate(tags)
            }
            Self::Or(left, right) => {
                left.evaluate(tags) || right.evaluate(tags)
            }
        }
    }
}

// ECMAScript WhiteSpace + LineTerminator used by the pinned tokenizer's /\s/.
fn javascript_whitespace(value: char) -> bool {
    matches!(value,
         '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn tokenize(input: &str) -> Result<Vec<String>, ExpressionError> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut escaped = false;
    for character in input.chars() {
        if escaped {
            if matches!(character, '(' | ')' | '\\')
                || javascript_whitespace(character)
            {
                token.push(character);
                escaped = false;
            } else {
                return Err(ExpressionError(format!(
                    "illegal escape before {character:?}"
                )));
            }
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '(' | ')')
            || javascript_whitespace(character)
        {
            if !token.is_empty() {
                tokens.push(std::mem::take(&mut token));
            }
            if !javascript_whitespace(character) {
                tokens.push(character.to_string());
            }
        } else {
            token.push(character);
        }
    }
    // The pinned JS tokenizer ignores a final escape marker; preserve that
    // observed contract rather than silently substituting a stricter grammar.
    if !token.is_empty() {
        tokens.push(token);
    }
    Ok(tokens)
}

struct ExpressionParser {
    tokens: Vec<String>,
    next: usize,
}
impl ExpressionParser {
    fn accept(&mut self, token: &str) -> bool {
        if self.tokens.get(self.next).is_some_and(|value| value == token) {
            self.next += 1;
            true
        } else {
            false
        }
    }
    fn or(&mut self) -> Result<Node, ExpressionError> {
        let mut node = self.and()?;
        while self.accept("or") {
            node = Node::Or(Box::new(node), Box::new(self.and()?));
        }
        Ok(node)
    }
    fn and(&mut self) -> Result<Node, ExpressionError> {
        let mut node = self.operand()?;
        while self.accept("and") {
            node = Node::And(Box::new(node), Box::new(self.operand()?));
        }
        Ok(node)
    }
    fn operand(&mut self) -> Result<Node, ExpressionError> {
        if self.accept("not") {
            return Ok(Node::Not(Box::new(self.operand()?)));
        }
        if self.accept("(") {
            let node = self.or()?;
            if !self.accept(")") {
                return Err(ExpressionError(
                    "unmatched opening parenthesis".into(),
                ));
            }
            return Ok(node);
        }
        let token = self
            .tokens
            .get(self.next)
            .ok_or_else(|| ExpressionError("expected an operand".into()))?;
        if matches!(token.as_str(), ")" | "and" | "or") {
            return Err(ExpressionError("expected an operand".into()));
        }
        let node = Node::Literal(token.clone());
        self.next += 1;
        Ok(node)
    }
}
