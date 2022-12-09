use std::{convert::Infallible, str::FromStr};

use cucumber::{given, Parameter, StatsWriter as _, World};

#[derive(Debug, Parameter, PartialEq)]
#[param(name = "param", regex = "'([^']*)'|(\\d+)")]
enum Param {
    Int(u64),
    Quoted(String),
}

impl FromStr for Param {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<u64>()
            .map_or_else(|_| Self::Quoted(s.to_owned()), Param::Int))
    }
}

#[given(regex = "^regex: int: (\\d+)$")]
#[given(expr = "expr: int: {param}")]
fn assert_int(_: &mut W, v: Param) {
    assert_eq!(v, Param::Int(42));
}

#[given(regex = "^regex: quoted: '([^']*)'$")]
#[given(expr = "expr: quoted: {param}")]
fn assert_quoted(_: &mut W, v: Param) {
    assert_eq!(v, Param::Quoted("inner".to_owned()));
}

#[derive(Clone, Copy, Debug, Default, World)]
struct W;

#[tokio::main]
async fn main() {
    let writer = W::cucumber()
        .run("tests/features/from_str_and_parameter")
        .await;

    assert_eq!(writer.passed_steps(), 4);
    assert_eq!(writer.skipped_steps(), 0);
    assert_eq!(writer.failed_steps(), 0);
    assert_eq!(writer.retried_steps(), 0);
    assert_eq!(writer.parsing_errors(), 0);
    assert_eq!(writer.hook_errors(), 0);
}
