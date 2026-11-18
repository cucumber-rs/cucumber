// Imports and executes only during completed Phase44 BDD acceptance.
// The actual pinned upstream classes implement all collection and formatting.
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { EventEmitter } from 'node:events';
import { PassThrough } from 'node:stream';
const require = createRequire(import.meta.url);
const root = dirname(require.resolve('@cucumber/cucumber/package.json'));
const RerunFormatter = require(join(root, 'lib/formatter/rerun_formatter.js')).default;
const EventDataCollector = require(join(root, 'lib/formatter/helpers/event_data_collector.js')).default;
const request = JSON.parse(readFileSync(0, 'utf8'));
const eventBroadcaster = new EventEmitter();
const eventDataCollector = new EventDataCollector(eventBroadcaster);
const logged = [];
const formatter = new RerunFormatter({
  eventBroadcaster, eventDataCollector,
  log: text => logged.push(text),
  parsedArgvOptions: { rerun: { separator: request.separator ?? '\n' } },
  cwd: process.cwd(), stream: new PassThrough(), cleanup: async () => {},
  colorFns: {}, snippetBuilder: {}, supportCodeLibrary: {},
});
for (const envelope of request.envelopes) eventBroadcaster.emit('envelope', envelope);
process.stdout.write(JSON.stringify({
  formatted: formatter.formatFailedTestCases(), logged,
  attempts: eventDataCollector.getTestCaseAttempts().map(attempt => ({
    attempt: attempt.attempt, status: attempt.worstTestStepResult.status,
    willBeRetried: attempt.willBeRetried, pickleId: attempt.pickle.id,
  })),
}));
