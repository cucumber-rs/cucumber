//! Locked official collaborators for acceptance scenarios.

use std::{path::Path, process::Command};

/// Launches an official oracle through its isolated, lock-pinned package root.
///
/// A phase-owned `CUCUMBER_ORACLE_ROOT` must already carry valid preparation
/// receipts. Ordinary CI prepares its own ignored root on first use instead.
pub fn command(script: impl AsRef<Path>) -> Command {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("node");
    command
        .env_remove("NODE_PATH")
        .env_remove("NODE_OPTIONS")
        .arg(repository.join("tests/support/oracle-launcher.mjs"))
        .arg(repository.join(script));
    command
}
