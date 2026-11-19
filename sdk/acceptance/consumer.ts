// Compiled from the installed declaration surface during SDK-010, then executed.
import { compile, capabilities, rerun, request, SdkError, type Compiled,
  type Capabilities, type RerunPlan, type SdkResponse } from 'cucumber-native-local';
const capabilitiesResult: Capabilities = capabilities();
if (capabilitiesResult.foreignStepExecution !== false) throw new Error('Unimplemented execution advertised');
const compiled: Compiled = compile({source:'Feature: TypeScript\n Scenario: works\n  Given native\n',
  uri:'typescript.feature', format:'gherkin', defaultLanguage:'en'});
const plan: RerunPlan = rerun({ndjson:'{"testRunStarted":{"id":"ts","timestamp":{"seconds":0,"nanos":0}}}\n{"testRunFinished":{"testRunStartedId":"ts","success":true,"timestamp":{"seconds":1,"nanos":0}}}\n', separator:'\n'});
if (plan.entries.length !== 0 || plan.runId !== 'ts') throw new Error('Typed rerun result changed');
const response: SdkResponse = JSON.parse(request('{"version":1,"operation":"capabilities","input":{}}'));
if (!response.ok) throw new Error(response.error.code);
let rejected = false;
try { compile({source:'not gherkin',uri:'typescript:error.feature',format:'gherkin',defaultLanguage:'en'}); }
catch (error: unknown) {
  if (!(error instanceof SdkError) || error.code !== 'SOURCE_ERROR' || error.uri !== 'typescript:error.feature') throw error;
  rejected = true;
}
if (!rejected) throw new Error('Typed source error was not thrown');
console.log(JSON.stringify(compiled));
