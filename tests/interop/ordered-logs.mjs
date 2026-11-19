// Invoked only by Phase37's completed ordered-retry BDD acceptance.
import { readFileSync } from 'node:fs';
import { Query } from '@cucumber/query';
const envelopes = readFileSync(0, 'utf8').trim().split('\n').map(JSON.parse);
const query = new Query();
for (const envelope of envelopes) query.update(envelope);
const describe = start => ({ name: query.findPickleBy(start).name,
  attempt: start.attempt, status: query.findMostSevereTestStepResultBy(start)?.status });
const attempts = envelopes.filter(e => e.testCaseFinished).map(({testCaseFinished: finished}) => {
  const start = query.findTestCaseStartedBy(finished);
  return { ...describe(start), willBeRetried: finished.willBeRetried,
    // Query16's attachment accessor is step-scoped. Native logs are case-scoped;
    // resolve the actual case via Query and retain that explicit scope.
    logs: envelopes.filter(e => e.attachment?.testCaseStartedId === start.id)
      .map(e => e.attachment.body) };
});
process.stdout.write(JSON.stringify({ attempts, finals: query.findAllTestCaseStarted().map(describe),
  counts: query.countMostSevereTestStepResultStatus() }));
