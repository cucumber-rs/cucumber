// Executed only by the completed Phase42 BDD boundary.
import fs from 'node:fs';
import { ParameterType, ParameterTypeRegistry } from '@cucumber/cucumber-expressions';
const { name } = JSON.parse(fs.readFileSync(0, 'utf8'));
try {
  const registry = new ParameterTypeRegistry();
  registry.defineParameterType(new ParameterType(name, 'value', String, value => value, false, false));
  process.stdout.write(JSON.stringify({ accepted: true }));
} catch (error) {
  process.stdout.write(JSON.stringify({ accepted: false, error: error.message }));
}
