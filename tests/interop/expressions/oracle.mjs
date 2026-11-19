// Invoked only by completed phase-end BDD acceptance.
import fs from 'node:fs';
import { CucumberExpression, ParameterTypeRegistry, ParameterType } from '@cucumber/cucumber-expressions';
import { parse } from '@cucumber/tag-expressions';
const request = JSON.parse(fs.readFileSync(0, 'utf8'));
try {
  if (request.kind === 'tags') {
    const expression = parse(request.expression);
    process.stdout.write(JSON.stringify({ valid: true, selected: expression.evaluate(request.tags) }));
  } else {
    const registry = new ParameterTypeRegistry();
    registry.defineParameterType(new ParameterType('color', /red|blue/, String, value => value, false, false));
    const expression = new CucumberExpression(request.expression, registry);
    const args = expression.match(request.text);
    process.stdout.write(JSON.stringify({ valid: true, matched: args !== null,
      values: args?.map(argument => { const value = argument.getValue(null); return typeof value === 'bigint' ? value.toString() : value; }) ?? [] }));
  }
} catch (error) {
  process.stdout.write(JSON.stringify({ valid: false, error: String(error.message) }));
}
