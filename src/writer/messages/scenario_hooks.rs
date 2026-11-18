//! Exact callback observations substituted for native composite stage envelopes.
use super::*;
use crate::runner::{
    ScenarioHookDefinition, ScenarioHookDisposition, ScenarioHookKind,
};

pub(super) struct ObservedHooks {
    pub steps: [Option<Vec<Value>>; 2],
    pub initialization: bool,
}
fn callback_id(definition: u64) -> String {
    format!("native-scenario-hook-{definition}")
}
pub(super) fn callback_step_id(case: &CaseRun, definition: u64) -> String {
    format!("{}-{}", case.plan.id, callback_id(definition))
}
pub(super) fn initialization_step_id(case: &CaseRun) -> String {
    format!("{}-native-world-initialization", case.plan.id)
}
const INITIALIZATION_ID: &str = "native-world-initialization";

impl<O: io::Write> Messages<O> {
    pub(super) fn observed_scenario_hooks(
        &mut self,
        case: &mut CaseRun,
    ) -> Result<ObservedHooks, Error> {
        let mut observed =
            ObservedHooks { steps: [None, None], initialization: false };
        let Some(store) = &self.scenario_hook_store else {
            return Ok(observed);
        };
        let Some(plan) = store.plan_for(
            std::path::Path::new(&case.uri),
            case.plan.rule.as_ref(),
            &case.plan.scenario,
            case.retries.map_or(0, |retry| retry.current),
        ) else {
            return Ok(observed);
        };
        let records = store.records_for(plan.attempt.scenario_id);
        let initializations = store
            .initializations()
            .into_iter()
            .filter(|i| i.attempt.scenario_id == plan.attempt.scenario_id)
            .collect::<Vec<_>>();
        if initializations.len() > 1 {
            return Err(Error::InvalidEvent(
                "duplicate observed World initialization",
            ));
        }
        if records.len() != plan.before.len() + plan.after.len() {
            return Err(Error::InvalidEvent(
                "scenario callback selection and observations disagree",
            ));
        }
        let mut used = HashSet::new();
        for (index, (enabled, definitions)) in
            [(plan.before_stage, &plan.before), (plan.after_stage, &plan.after)]
                .into_iter()
                .enumerate()
        {
            if !enabled {
                if !definitions.is_empty() {
                    return Err(Error::InvalidEvent(
                        "replaced hook stage has selected callbacks",
                    ));
                }
                continue;
            }
            if !case.hooks[index] {
                return Err(Error::InvalidEvent(
                    "observed registry stage has no native lifecycle",
                ));
            }
            let mut steps = Vec::new();
            let mut events = Vec::new();
            if index == 0 {
                // Initialization is observed every registry setup attempt. Keeping its
                // actual slot in each plan also keeps retry TestCase shape stable.
                if let Some(initialization) = initializations.first() {
                    observed.initialization = true;
                    if !self.emitted_initialization {
                        self.emit(json!({"hook":{"id":INITIALIZATION_ID,"name":"Native World initialization",
                            "type":"BEFORE_TEST_CASE","sourceReference":{}}}))?;
                        self.emitted_initialization = true;
                    }
                    let id = initialization_step_id(case);
                    steps.push(json!({"id":id,"hookId":INITIALIZATION_ID}));
                    events.extend(observed_events(
                        case,
                        &id,
                        initialization.started_at,
                        initialization.finished_at,
                        &initialization.result,
                        None,
                    )?);
                } else {
                    return Err(Error::InvalidEvent(
                        "observed registry setup has no World initialization record",
                    ));
                }
            }
            for definition in definitions {
                if !used.insert(definition.id) {
                    return Err(Error::InvalidEvent(
                        "duplicate selected scenario callback identity",
                    ));
                }
                let matching = records
                    .iter()
                    .filter(|record| record.definition.id == definition.id)
                    .collect::<Vec<_>>();
                if matching.len() != 1 {
                    return Err(Error::InvalidEvent(
                        "selected scenario callback has no unique observation",
                    ));
                }
                let record = matching[0];
                if record.definition.name != definition.name
                    || record.definition.kind != definition.kind
                    || record.definition.location != definition.location
                {
                    return Err(Error::InvalidEvent(
                        "scenario callback registration changed during execution",
                    ));
                }
                self.scenario_hook_definition(definition)?;
                let id = callback_step_id(case, definition.id);
                steps
                    .push(json!({"id":id,"hookId":callback_id(definition.id)}));
                match &record.disposition {
                    ScenarioHookDisposition::Executed {
                        started_at,
                        finished_at,
                        result,
                        outcome,
                        ..
                    } => events.extend(observed_events(
                        case,
                        &id,
                        *started_at,
                        *finished_at,
                        result,
                        *outcome,
                    )?),
                    ScenarioHookDisposition::Skipped { at, .. } => {
                        events.push(json!({"testStepStarted":{"testCaseStartedId":case.started_id,"testStepId":id,"timestamp":timestamp(*at)?}}));
                        events.push(json!({"testStepFinished":{"testCaseStartedId":case.started_id,"testStepId":id,"timestamp":timestamp(*at)?,
                            "testStepResult":{"status":"SKIPPED","duration":duration(Duration::ZERO)}}}));
                    }
                }
            }
            let composite = hook_step_id(case, index);
            let mut replaced = false;
            let mut finished = false;
            let mut updated = Vec::new();
            for envelope in std::mem::take(&mut case.events) {
                if envelope
                    .get("testStepStarted")
                    .is_some_and(|start| start["testStepId"] == composite)
                {
                    if replaced {
                        return Err(Error::InvalidEvent(
                            "duplicate composite stage start",
                        ));
                    }
                    replaced = true;
                    updated.append(&mut events);
                } else if envelope
                    .get("testStepFinished")
                    .is_some_and(|end| end["testStepId"] == composite)
                {
                    if finished {
                        return Err(Error::InvalidEvent(
                            "duplicate composite stage finish",
                        ));
                    }
                    finished = true;
                } else {
                    updated.push(envelope)
                }
            }
            if !replaced || !finished {
                return Err(Error::InvalidEvent(
                    "incomplete composite stage for observed callbacks",
                ));
            }
            case.events = updated;
            case.hooks[index] = false;
            observed.steps[index] = Some(steps);
        }
        if !plan.before_stage && !initializations.is_empty() {
            return Err(Error::InvalidEvent(
                "initialization record belongs to replaced setup stage",
            ));
        }
        Ok(observed)
    }
    fn scenario_hook_definition(
        &mut self,
        definition: &ScenarioHookDefinition,
    ) -> Result<(), Error> {
        if self.emitted_scenario_hooks.insert(definition.id) {
            self.emit(json!({"hook":{"id":callback_id(definition.id),"name":definition.name,
                "type":match definition.kind{ScenarioHookKind::Before=>"BEFORE_TEST_CASE",ScenarioHookKind::After=>"AFTER_TEST_CASE"},
                "sourceReference":{"uri":definition.location.path,"location":{"line":definition.location.line,"column":definition.location.column}}}}))?;
        }
        Ok(())
    }
}
fn observed_events(
    case: &CaseRun,
    id: &str,
    start: SystemTime,
    end: SystemTime,
    result: &Result<(), String>,
    outcome: Option<crate::execution::Outcome>,
) -> Result<Vec<Value>, Error> {
    let elapsed = end.duration_since(start).map_err(|_| {
        Error::InvalidEvent("observed callback finished before it started")
    })?;
    let mut outcome = json!({"status":outcome.map_or(if result.is_ok(){"PASSED"}else{"FAILED"}, |value| outcome_status(value.status)),"duration":duration(elapsed)});
    if let Err(message) = result {
        outcome["message"] = json!(message)
    }
    Ok(vec![
        json!({"testStepStarted":{"testCaseStartedId":case.started_id,"testStepId":id,"timestamp":timestamp(start)?}}),
        json!({"testStepFinished":{"testCaseStartedId":case.started_id,"testStepId":id,"timestamp":timestamp(end)?,"testStepResult":outcome}}),
    ])
}
