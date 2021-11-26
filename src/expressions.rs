//! TODO

pub use cucumber_codegen::Parameter;

/// TODO
pub trait Parameter {
    /// TODO
    const REGEX: &'static str;

    /// TODO
    const NAME: &'static str;
}
