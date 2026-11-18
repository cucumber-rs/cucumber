//! Internal callback aggregation; normal completion requests are not panics.
use crate::execution::{Completion, Outcome, OutcomePolicy, OutcomeStatus};

pub(crate) fn status<T>(
    result: &Result<(), T>,
    completion: Option<Completion>,
) -> OutcomeStatus {
    if result.is_err() {
        OutcomeStatus::Failed
    } else {
        completion.map_or(OutcomeStatus::Passed, Into::into)
    }
}
pub(crate) fn metadata(
    policy: OutcomePolicy,
    status: OutcomeStatus,
) -> Option<Outcome> {
    (policy != OutcomePolicy::Legacy).then(|| policy.outcome(status))
}
#[derive(Debug)]
pub(crate) struct Summary {
    pub(crate) status: OutcomeStatus,
    pub(crate) failures: Vec<String>,
}
impl Default for Summary {
    fn default() -> Self {
        Self { status: OutcomeStatus::Passed, failures: Vec::new() }
    }
}
impl Summary {
    pub(crate) fn merge(&mut self, other: Self) {
        self.status = self.status.max(other.status);
        self.failures.extend(other.failures);
    }
    pub(crate) fn diagnostic(&self, stage: &str) -> String {
        if self.failures.is_empty() {
            format!("{stage} requested {:?}", self.status)
        } else {
            self.failures.join("; ")
        }
    }
}
