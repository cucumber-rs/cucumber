//! Formatting utilities for Basic writer output.

use std::{
    borrow::Cow,
    cmp, env, fmt::Write,
    sync::LazyLock,
};

use itertools::Itertools as _;
use regex::CaptureLocations;

use crate::event::Info;

/// Coerces error information into a readable string.
pub fn coerce_error(err: &Info) -> Cow<'static, str> {
    (**err)
        .downcast_ref::<String>()
        .map(|s| s.clone().into())
        .or_else(|| (**err).downcast_ref::<&str>().map(|s| s.to_owned().into()))
        .unwrap_or_else(|| "(Could not resolve panic payload)".into())
}

/// Formats the given [`str`] by adding `indent`s to each line to prettify the
/// output.
pub(super) fn format_str_with_indent(str: impl AsRef<str>, indent: usize) -> String {
    let str = str
        .as_ref()
        .lines()
        .map(|line| format!("{}{line}", " ".repeat(indent)))
        .join("\n");
    if str.is_empty() { String::new() } else { format!("\n{str}") }
}

/// Formats the given [`gherkin::Table`] and adds `indent`s to each line to
/// prettify the output.
pub(super) fn format_table(table: &gherkin::Table, indent: usize) -> String {
    let max_row_len = table
        .rows
        .iter()
        .fold(None, |mut acc: Option<Vec<_>>, row| {
            if let Some(existing_len) = acc.as_mut() {
                for (cell, max_len) in row.iter().zip(existing_len) {
                    *max_len = cmp::max(*max_len, cell.len());
                }
            } else {
                acc = Some(row.iter().map(String::len).collect::<Vec<_>>());
            }
            acc
        })
        .unwrap_or_default();

    let mut table = table
        .rows
        .iter()
        .map(|row| {
            row.iter().zip(&max_row_len).fold(
                String::new(),
                |mut out, (cell, len)| {
                    _ = write!(out, "| {cell:len$} ");
                    out
                },
            )
        })
        .map(|row| format!("{}{row}", " ".repeat(indent + 1)))
        .join("|\n");

    if !table.is_empty() {
        table.insert(0, '\n');
        table.push('|');
    }

    table
}

/// Formats `value`s in the given `captures` with the provided `accent` style
/// and with the `default` style anything else.
pub(super) fn format_captures<D, A>(
    value: impl AsRef<str>,
    captures: &CaptureLocations,
    default: D,
    accent: A,
) -> String
where
    D: for<'a> Fn(&'a str) -> Cow<'a, str>,
    A: for<'a> Fn(&'a str) -> Cow<'a, str>,
{
    #![expect( // intentional
        clippy::string_slice,
        reason = "all indices are obtained from the source string"
    )]

    let value = value.as_ref();

    let (mut formatted, end) =
        (1..captures.len()).filter_map(|group| captures.get(group)).fold(
            (String::with_capacity(value.len()), 0),
            |(mut str, old), (start, end)| {
                // Ignore nested groups.
                if old > start {
                    return (str, old);
                }

                str.push_str(&default(&value[old..start]));
                str.push_str(&accent(&value[start..end]));
                (str, end)
            },
        );
    formatted.push_str(&default(&value[end..value.len()]));

    formatted
}

/// Trims start of the path if it matches the current project directory.
pub fn trim_path(path: &str) -> &str {
    /// Path of the current project directory.
    static CURRENT_DIR: LazyLock<String> = LazyLock::new(|| {
        env::var("CARGO_WORKSPACE_DIR")
            .or_else(|_| env::var("CARGO_MANIFEST_DIR"))
            .unwrap_or_else(|_| {
                env::current_dir()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
            })
    });

    path.trim_start_matches(&**CURRENT_DIR)
        .trim_start_matches('/')
        .trim_start_matches('\\')
}

