// Official Query executes only from the focused Phase27 BDD boundary.
import { readFileSync } from 'node:fs';
import { Query } from '@cucumber/query';
const envelopes = readFileSync(0, 'utf8').trim().split('\n').map(JSON.parse);
const query = new Query();
for (const envelope of envelopes) query.update(envelope);
// Query.findAllTestCaseStarted intentionally excludes attempts marked for retry.
// Resolve every observed start through Query to inspect stable definition IDs.
const attempts = envelopes.filter(envelope => envelope.testCaseStarted).map(envelope => envelope.testCaseStarted).map(start => ({
  id: start.id,
  attempt: start.attempt,
  steps: query.findTestStepFinishedAndTestStepBy(start).filter(([, step]) => step.pickleStepId).map(([finished, step]) => ({
    text: query.findPickleStepBy(step).text,
    status: finished.testStepResult.status,
    ids: step.stepDefinitionIds,
    definitions: query.findStepDefinitionsBy(step),
    unambiguous: query.findUnambiguousStepDefinitionBy(step),
  })),
}));
process.stdout.write(JSON.stringify({ definitions: query.findAllStepDefinitions(), attempts }));
