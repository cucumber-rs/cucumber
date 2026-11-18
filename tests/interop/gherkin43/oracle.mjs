// Pinned official Gherkin parser/compiler; executed only by phase-end BDD.
import fs from 'node:fs'
import { generateMessages } from '@cucumber/gherkin'
import { IdGenerator, SourceMediaType } from '@cucumber/messages'
const source = fs.readFileSync(0, 'utf8')
const uri = process.argv[2] || 'official.feature'
const envelopes = generateMessages(source, uri, SourceMediaType.TEXT_X_CUCUMBER_GHERKIN_PLAIN, {
  includeSource: false,
  includeGherkinDocument: true,
  includePickles: true,
  newId: IdGenerator.incrementing(),
})
const errors = envelopes.filter(envelope => envelope.parseError)
if (errors.length) {
  process.stderr.write(JSON.stringify(errors))
  process.exitCode = 1
} else {
  process.stdout.write(JSON.stringify({
    gherkinDocument: envelopes.find(envelope => envelope.gherkinDocument)?.gherkinDocument,
    pickles: envelopes.filter(envelope => envelope.pickle).map(envelope => envelope.pickle),
  }))
}
