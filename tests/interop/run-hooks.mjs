// Invoked exclusively by phase-end BDD acceptance.
import { readFileSync } from 'node:fs';
import { Query } from '@cucumber/query';
const query = new Query();
for (const line of readFileSync(0, 'utf8').trim().split('\n')) query.update(JSON.parse(line));
process.stdout.write(JSON.stringify(query.findAllTestRunHookFinished().map(finished => ({
  hook: query.findHookBy(finished), started: query.findTestRunHookStartedBy(finished),
  result: finished.result
}))));
