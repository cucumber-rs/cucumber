// Called only by Phase23 focused BDD acceptance.
import { readFileSync } from 'node:fs';
import { Query } from '@cucumber/query';
const envelopes = readFileSync(0, 'utf8').trim().split('\n').map(JSON.parse);
const query = new Query();
for (const envelope of envelopes) query.update(envelope);
const attempts = envelopes.filter(e => e.testCaseStarted).map(({ testCaseStarted: start }) => ({
  name: query.findPickleBy(start).name,
  attempt: start.attempt,
  steps: query.findTestStepFinishedAndTestStepBy(start).map(([finished, step]) => ({
    text: query.findPickleStepBy(step)?.text,
    hookType: step.hookId ? query.findHookBy(step)?.type : undefined,
    status: finished.testStepResult.status,
    attachments: query.findAttachmentsBy(finished).map(attachment => ({
      ...attachment,
      decoded: attachment.contentEncoding === 'BASE64'
        ? Array.from(Buffer.from(attachment.body, 'base64')) : undefined
    }))
  }))
}));
process.stdout.write(JSON.stringify({ attempts }));
