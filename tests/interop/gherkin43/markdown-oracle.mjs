// The official Markdown oracle runs only within completed phase-end BDD.
import fs from 'node:fs'
import { generateMessages } from '@cucumber/gherkin'
import { IdGenerator, SourceMediaType } from '@cucumber/messages'
const source = fs.readFileSync(0, 'utf8')
const envelopes = generateMessages(source, process.argv[2] || 'boundary.feature.md',
  SourceMediaType.TEXT_X_CUCUMBER_GHERKIN_MARKDOWN, {
    defaultDialect: process.argv[3] || 'en', includeSource: false,
    includeGherkinDocument: true, includePickles: true, newId: IdGenerator.incrementing(),
  })
const errors = envelopes.filter(e => e.parseError)
if (errors.length) {
  process.stderr.write(JSON.stringify(errors)); process.exitCode = 1
} else {
  process.stdout.write(JSON.stringify({gherkinDocument: envelopes.find(e => e.gherkinDocument)?.gherkinDocument,
    pickles: envelopes.filter(e => e.pickle).map(e => e.pickle)}))
}
