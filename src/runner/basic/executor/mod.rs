//! Modularized executor implementation following Single Responsibility Principle.
//!
//! This module breaks down the large executor implementation into focused components:
//! - `core`: Main Executor struct and constructor
//! - `hooks`: Before/after hook execution logic
//! - `steps`: Step execution logic
//! - `events`: Event sending functionality

mod core;
mod events;
mod hooks;
mod steps;

pub use core::Executor;

// Re-export for backward compatibility
pub use hooks::HookExecutor;
pub use steps::StepExecutor;
pub use events::EventSender;