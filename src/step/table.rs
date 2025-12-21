// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Step table parameter support.
//!
//! This module provides utilities for extracting and working with
//! data tables in step functions.

use crate::data_table::DataTable;

/// Trait for extracting a DataTable from a step.
///
/// This trait allows step functions to receive tables as parameters
/// through automatic extraction from the step context.
pub trait FromStep {
    /// Extracts a value from a step.
    fn from_step(step: &gherkin::Step) -> Option<Self>
    where
        Self: Sized;
}

impl FromStep for DataTable {
    fn from_step(step: &gherkin::Step) -> Option<Self> {
        step.table.as_ref().map(DataTable::from)
    }
}

impl FromStep for Option<DataTable> {
    fn from_step(step: &gherkin::Step) -> Option<Self> {
        Some(DataTable::from_step(step))
    }
}

/// Helper function to extract a DataTable from a step.
///
/// # Example
///
/// ```rust,ignore
/// use cucumber::{given, gherkin::Step};
/// use cucumber::step::table::extract_table;
///
/// #[given("a list of items")]
/// async fn items(world: &mut World, step: &Step) {
///     if let Some(table) = extract_table(step) {
///         for item in table.hashes() {
///             // Process items
///         }
///     }
/// }
/// ```
#[must_use]
pub fn extract_table(step: &gherkin::Step) -> Option<DataTable> {
    DataTable::from_step(step)
}

/// Macro to simplify table extraction in step functions.
///
/// # Example
///
/// ```rust,ignore
/// use cucumber::{given, gherkin::Step, table_from_step};
///
/// #[given("a list of items")]
/// async fn items(world: &mut World, step: &Step) {
///     table_from_step!(step, table, {
///         for item in table.hashes() {
///             // Process items
///         }
///     });
/// }
/// ```
#[macro_export]
macro_rules! table_from_step {
    ($step:expr, $table:ident, $body:block) => {
        if let Some($table) = $crate::step::table::extract_table($step) {
            $body
        }
    };
}