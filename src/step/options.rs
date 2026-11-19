//! Additive per-registration options shared by timeout and metadata phases.

/// Options belonging to one live step registration.
///
/// Existing insertion methods use defaults and clear any replaced entry's
/// options. Use the explicit insertion methods to retain supplied options.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct DefinitionOptions {
    pub(crate) timeout: Option<crate::runner::TimeoutPolicy>,
    pub(crate) expression: Option<crate::support_code::ExpressionMetadata>,
    pub(crate) regular_expression: Option<crate::support_code::RegexMetadata>,
}

impl DefinitionOptions {
    /// Creates options inheriting runner timeout policy and without expression
    /// metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the runner timeout for this registration.
    #[must_use]
    pub fn timeout(mut self, policy: crate::runner::TimeoutPolicy) -> Self {
        self.timeout = Some(policy);
        self
    }

    /// Returns the explicit timeout policy, or `None` to inherit the runner
    /// policy.
    #[must_use]
    pub const fn timeout_policy(&self) -> Option<crate::runner::TimeoutPolicy> {
        self.timeout
    }

    /// Retains actual original expression and validated capture-layout
    /// metadata.
    #[must_use]
    pub fn expression(
        mut self,
        metadata: crate::support_code::ExpressionMetadata,
    ) -> Self {
        self.expression = Some(metadata);
        self.regular_expression = None;
        self
    }

    /// Declares an explicit raw-regex capture mapping without inferring types.
    ///
    /// Selecting raw metadata replaces any retained expression metadata.
    #[must_use]
    pub fn regular_expression(
        mut self,
        metadata: crate::support_code::RegexMetadata,
    ) -> Self {
        self.regular_expression = Some(metadata);
        self.expression = None;
        self
    }

    /// Returns an explicit raw capture mapping, if supplied.
    #[must_use]
    pub fn regular_expression_metadata(
        &self,
    ) -> Option<&crate::support_code::RegexMetadata> {
        self.regular_expression.as_ref()
    }

    /// Returns the retained original expression metadata, if available.
    #[must_use]
    pub fn expression_metadata(
        &self,
    ) -> Option<&crate::support_code::ExpressionMetadata> {
        self.expression.as_ref()
    }
}
