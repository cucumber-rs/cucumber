//! Thin native adapter: all compilation and rerun semantics live in Rust's SDK.

use pyo3::prelude::*;

/// Pass an unchanged JSON request to a fresh shared SDK instance.
#[pyfunction]
fn request(raw: &str) -> String {
    cucumber_sdk::Sdk::new().request(raw)
}

/// The symbol name matches the mixed Python package's configured submodule.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(request, module)?)
}
