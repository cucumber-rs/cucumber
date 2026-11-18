// Invoked only by the phase-end Rust BDD acceptance, not a separate test suite.
import { readFileSync } from 'node:fs';
import { Query } from '@cucumber/query';
import { JUnitXmlPrinter } from '@cucumber/junit-xml-formatter';
const envelopes = readFileSync(0, 'utf8').trim().split('\n').map(JSON.parse);
const query = new Query();
let xml = '';
const printer = new JUnitXmlPrinter({}, content => { xml += content; });
for (const envelope of envelopes) { query.update(envelope); printer.update(envelope); }
const outcomes = query.findAllTestCaseStarted().map(start => {
  const pickle = query.findPickleBy(start);
  const lineage = query.findLineageBy(pickle);
  const result = query.findMostSevereTestStepResultBy(start);
  return {
    name: pickle.name,
    attempt: start.attempt,
    status: result?.status,
    feature: lineage.feature?.name,
    rule: lineage.rule?.name,
    exampleLine: lineage.example?.location.line,
    tags: pickle.tags.map(tag => tag.name),
    steps: query.findTestStepFinishedAndTestStepBy(start).map(([finished, step]) => {
      const pickleStep = query.findPickleStepBy(step);
      if (pickleStep) {
        const ast = query.findStepBy(pickleStep);
        return { text: pickleStep.text, argument: pickleStep.argument, sourceText: ast.text, status: finished.testStepResult.status };
      }
      const hook = query.findHookBy(step);
      return { hookType: hook?.type, status: finished.testStepResult.status };
    })
  };
});
process.stdout.write(JSON.stringify({ outcomes, counts: query.countMostSevereTestStepResultStatus(), xml,
  observedAttempts: envelopes.filter(e => e.testCaseStarted).length }));
