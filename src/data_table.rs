// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Data table support for Cucumber steps.
//!
//! This module provides a [`DataTable`] type that offers a rich API
//! for working with Gherkin data tables, similar to cucumber-js.

use std::collections::HashMap;
use std::fmt;

/// A data table from a Gherkin step.
///
/// Provides convenience methods for accessing table data in various formats.
///
/// # Example
///
/// ```rust
/// use cucumber::DataTable;
///
/// let table = DataTable::from(vec![
///     vec!["name", "age"],
///     vec!["Alice", "30"],
///     vec!["Bob", "25"],
/// ]);
///
/// // Get as array of hashmaps
/// let hashes = table.hashes();
/// assert_eq!(hashes[0].get("name"), Some(&"Alice".to_string()));
///
/// // Get raw data
/// let raw = table.raw();
/// assert_eq!(raw[0], vec!["name", "age"]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DataTable {
    rows: Vec<Vec<String>>,
}

impl DataTable {
    /// Creates a new [`DataTable`] from a vector of rows.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::DataTable;
    ///
    /// let table = DataTable::new(vec![
    ///     vec!["header1".to_string(), "header2".to_string()],
    ///     vec!["value1".to_string(), "value2".to_string()],
    /// ]);
    /// ```
    #[must_use]
    pub fn new(rows: Vec<Vec<String>>) -> Self {
        Self { rows }
    }

    /// Creates a [`DataTable`] from a Gherkin table.
    #[must_use]
    pub fn from_gherkin(table: &gherkin::Table) -> Self {
        Self {
            rows: table.rows.clone(),
        }
    }

    /// Returns the raw table data as a 2D vector.
    ///
    /// Includes all rows including the header row (if present).
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::DataTable;
    ///
    /// let table = DataTable::from(vec![
    ///     vec!["name", "age"],
    ///     vec!["Alice", "30"],
    /// ]);
    ///
    /// let raw = table.raw();
    /// assert_eq!(raw.len(), 2);
    /// assert_eq!(raw[0], vec!["name", "age"]);
    /// ```
    #[must_use]
    pub fn raw(&self) -> Vec<Vec<String>> {
        self.rows.clone()
    }

    /// Returns the table rows without the header row.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::DataTable;
    ///
    /// let table = DataTable::from(vec![
    ///     vec!["name", "age"],
    ///     vec!["Alice", "30"],
    ///     vec!["Bob", "25"],
    /// ]);
    ///
    /// let rows = table.rows();
    /// assert_eq!(rows.len(), 2);
    /// assert_eq!(rows[0], vec!["Alice", "30"]);
    /// ```
    #[must_use]
    pub fn rows(&self) -> Vec<Vec<String>> {
        if self.rows.is_empty() {
            Vec::new()
        } else {
            self.rows[1..].to_vec()
        }
    }

    /// Converts the table to an array of hashmaps.
    ///
    /// Uses the first row as keys for the hashmaps.
    /// Each subsequent row becomes a hashmap with the header values as keys.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::DataTable;
    ///
    /// let table = DataTable::from(vec![
    ///     vec!["name", "age", "city"],
    ///     vec!["Alice", "30", "NYC"],
    ///     vec!["Bob", "25", "LA"],
    /// ]);
    ///
    /// let hashes = table.hashes();
    /// assert_eq!(hashes[0].get("name"), Some(&"Alice".to_string()));
    /// assert_eq!(hashes[1].get("age"), Some(&"25".to_string()));
    /// ```
    #[must_use]
    pub fn hashes(&self) -> Vec<HashMap<String, String>> {
        if self.rows.is_empty() {
            return Vec::new();
        }

        let headers = &self.rows[0];
        self.rows[1..]
            .iter()
            .map(|row| {
                headers
                    .iter()
                    .zip(row.iter())
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .collect()
    }

    /// Converts a two-column table into a hashmap.
    ///
    /// The first column becomes the keys and the second column becomes the values.
    ///
    /// # Errors
    ///
    /// Returns `None` if any row doesn't have exactly 2 columns.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::DataTable;
    ///
    /// let table = DataTable::from(vec![
    ///     vec!["setting", "value"],
    ///     vec!["timeout", "30"],
    ///     vec!["retries", "3"],
    /// ]);
    ///
    /// let hash = table.rows_hash().unwrap();
    /// assert_eq!(hash.get("timeout"), Some(&"30".to_string()));
    /// ```
    #[must_use]
    pub fn rows_hash(&self) -> Option<HashMap<String, String>> {
        let mut result = HashMap::new();
        
        for row in &self.rows {
            if row.len() != 2 {
                return None;
            }
            result.insert(row[0].clone(), row[1].clone());
        }
        
        Some(result)
    }

    /// Returns a transposed version of the table.
    ///
    /// Rows become columns and columns become rows.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::DataTable;
    ///
    /// let table = DataTable::from(vec![
    ///     vec!["name", "Alice", "Bob"],
    ///     vec!["age", "30", "25"],
    /// ]);
    ///
    /// let transposed = table.transpose();
    /// assert_eq!(transposed.raw()[0], vec!["name", "age"]);
    /// assert_eq!(transposed.raw()[1], vec!["Alice", "30"]);
    /// ```
    #[must_use]
    pub fn transpose(&self) -> Self {
        if self.rows.is_empty() {
            return Self::new(Vec::new());
        }

        let width = self.rows[0].len();
        let mut transposed = vec![Vec::new(); width];

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < width {
                    transposed[i].push(cell.clone());
                }
            }
        }

        Self::new(transposed)
    }

    /// Returns only the specified columns from the table.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::DataTable;
    ///
    /// let table = DataTable::from(vec![
    ///     vec!["name", "age", "city"],
    ///     vec!["Alice", "30", "NYC"],
    ///     vec!["Bob", "25", "LA"],
    /// ]);
    ///
    /// let subset = table.columns(&["name", "city"]);
    /// assert_eq!(subset.raw()[0], vec!["name", "city"]);
    /// assert_eq!(subset.raw()[1], vec!["Alice", "NYC"]);
    /// ```
    #[must_use]
    pub fn columns(&self, column_names: &[&str]) -> Self {
        if self.rows.is_empty() {
            return Self::new(Vec::new());
        }

        let headers = &self.rows[0];
        let indices: Vec<usize> = column_names
            .iter()
            .filter_map(|name| {
                headers.iter().position(|h| h == name)
            })
            .collect();

        let new_rows = self.rows
            .iter()
            .map(|row| {
                indices
                    .iter()
                    .filter_map(|&i| row.get(i).cloned())
                    .collect()
            })
            .collect();

        Self::new(new_rows)
    }

    /// Checks if the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the number of rows in the table (including header).
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns the width (number of columns) of the table.
    ///
    /// Returns 0 if the table is empty.
    #[must_use]
    pub fn width(&self) -> usize {
        self.rows.first().map_or(0, |row| row.len())
    }
}

impl From<Vec<Vec<&str>>> for DataTable {
    fn from(rows: Vec<Vec<&str>>) -> Self {
        let string_rows = rows
            .into_iter()
            .map(|row| row.into_iter().map(String::from).collect())
            .collect();
        Self::new(string_rows)
    }
}

impl From<Vec<Vec<String>>> for DataTable {
    fn from(rows: Vec<Vec<String>>) -> Self {
        Self::new(rows)
    }
}

impl From<&gherkin::Table> for DataTable {
    fn from(table: &gherkin::Table) -> Self {
        Self::from_gherkin(table)
    }
}

impl fmt::Display for DataTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.rows {
            writeln!(f, "| {} |", row.join(" | "))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw() {
        let table = DataTable::from(vec![
            vec!["name", "age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);
        
        let raw = table.raw();
        assert_eq!(raw.len(), 3);
        assert_eq!(raw[0], vec!["name", "age"]);
        assert_eq!(raw[1], vec!["Alice", "30"]);
    }

    #[test]
    fn test_rows() {
        let table = DataTable::from(vec![
            vec!["name", "age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);
        
        let rows = table.rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["Alice", "30"]);
        assert_eq!(rows[1], vec!["Bob", "25"]);
    }

    #[test]
    fn test_hashes() {
        let table = DataTable::from(vec![
            vec!["name", "age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);
        
        let hashes = table.hashes();
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes[0].get("name"), Some(&"Alice".to_string()));
        assert_eq!(hashes[0].get("age"), Some(&"30".to_string()));
        assert_eq!(hashes[1].get("name"), Some(&"Bob".to_string()));
    }

    #[test]
    fn test_rows_hash() {
        let table = DataTable::from(vec![
            vec!["setting", "value"],
            vec!["timeout", "30"],
            vec!["retries", "3"],
        ]);
        
        let hash = table.rows_hash().unwrap();
        assert_eq!(hash.get("timeout"), Some(&"30".to_string()));
        assert_eq!(hash.get("retries"), Some(&"3".to_string()));
    }

    #[test]
    fn test_rows_hash_invalid() {
        let table = DataTable::from(vec![
            vec!["a", "b", "c"],
            vec!["1", "2", "3"],
        ]);
        
        assert!(table.rows_hash().is_none());
    }

    #[test]
    fn test_transpose() {
        let table = DataTable::from(vec![
            vec!["name", "Alice", "Bob"],
            vec!["age", "30", "25"],
        ]);
        
        let transposed = table.transpose();
        assert_eq!(transposed.raw()[0], vec!["name", "age"]);
        assert_eq!(transposed.raw()[1], vec!["Alice", "30"]);
        assert_eq!(transposed.raw()[2], vec!["Bob", "25"]);
    }

    #[test]
    fn test_columns() {
        let table = DataTable::from(vec![
            vec!["name", "age", "city"],
            vec!["Alice", "30", "NYC"],
            vec!["Bob", "25", "LA"],
        ]);
        
        let subset = table.columns(&["name", "city"]);
        assert_eq!(subset.raw()[0], vec!["name", "city"]);
        assert_eq!(subset.raw()[1], vec!["Alice", "NYC"]);
        assert_eq!(subset.raw()[2], vec!["Bob", "LA"]);
    }
}