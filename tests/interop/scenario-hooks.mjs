// Used only by completed Phase31 BDD acceptance.
import { readFileSync } from 'node:fs';
import { Query } from '@cucumber/query';
const envelopes = readFileSync(0, 'utf8').trim().split('\n').map(JSON.parse);
const query = new Query();
for (const envelope of envelopes) query.update(envelope);
const attempts = envelopes.filter(e => e.testCaseStarted).map(({testCaseStarted:start}) => ({
  name: query.findPickleBy(start).name, attempt: start.attempt,
  steps: query.findTestStepFinishedAndTestStepBy(start).map(([finished,step]) => ({
    hook: step.hookId ? query.findHookBy(step) : undefined,
    status: finished.testStepResult.status, attachments: query.findAttachmentsBy(finished)
  }))
}));
process.stdout.write(JSON.stringify({attempts,success:query.findTestRunFinished().success}));
