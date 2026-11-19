// Executed only by the completed Phase39 BDD boundary.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
const [executable, fixture, archive] = process.argv.slice(2);
const repository = process.cwd();
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'cucumber-vlbv-'));
const digest = value => crypto.createHash('sha256').update(value).digest('hex');
const put = (name, value) => {
  const bytes = typeof value === 'string' || Buffer.isBuffer(value) ? value : `${JSON.stringify(value, null, 2)}\n`;
  const file = path.join(root, name); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, bytes);
  return { path: name, sha256: digest(bytes) };
};
const load = ref => JSON.parse(fs.readFileSync(path.join(root, ref.path)));
const env = { ...process.env };
const baseline = Date.now();
const interleaved = fixture === 'valid interleaved lifecycle';
const originals = [];
try {
  try { fs.linkSync(executable, path.join(root, 'application')); }
  catch (error) { if (error.code !== 'EXDEV') throw error; fs.copyFileSync(executable, path.join(root, 'application'), fs.constants.COPYFILE_FICLONE); }
  const build = { path: 'application', sha256: digest(fs.readFileSync(executable)) };
  const source = put('production.mjs', fs.readFileSync(path.join(repository, 'scripts/messages-to-boundary-results.mjs')));
  const truth = put('independent-truth.txt', 'expected\n');
  const modes = fixture.includes('repair') ? [fixture === 'repair after unexercised cleanup' ? 'pass' : 'fail', 'pass'] : [fixture === 'nonzero green process' ? 'exit1' : ['failed global completion', 'concealed global failure'].includes(fixture) ? 'global' : fixture === 'product mismatch' ? 'fail' : ['failed then successful retry', 'duplicate attempt number', 'unpromised successor'].includes(fixture) ? 'retry' : fixture === 'dry run' ? 'dry' : fixture === 'process exit two' ? 'exit2' : fixture === 'skipped step' ? 'skipped' : fixture === 'undefined step' ? 'undefined' : 'pass'];
  const features = modes.map((mode, i) => put(`input-${i}.feature`, `Feature: Real evidence\n  Scenario: Actual operation\n    Given ${mode === 'undefined' ? 'undefined observation' : `observe ${mode === 'skipped' ? 'fail' : mode}`}\n    Then check the observation\n${fixture === 'unmapped failing attempt' ? '  Scenario: Unmapped failure\n    Given observe fail\n    Then check the observation\n' : interleaved ? '  Scenario: Other actual operation\n    Given observe pass\n    Then check the observation\n' : ''}`));
  const controlFeature = put('negative.feature', 'Feature: Oracle sensitivity\n  Scenario: Plausible wrong observation\n    Given observe fail\n    Then check the observation\n');
  const candidate = put('candidate.json', { sourceDigests: { [source.path]: source.sha256 }, build: [build], configuration: [], fixtures: [...features, controlFeature], references: [truth] });
  const identity = { projectId: '00000000-0000-4000-8000-000000000039', runId: 'phase39-run', phaseId: 'phase39-fixture', planRevision: null, contractDigest: null };
  const oracle = { observation: 'Actual observed value', expectedTruthSource: 'independent-truth.txt', comparator: 'real handler assert_eq(actual, expected)' };
  const scenario = { id: 'VLBV-1', name: 'Real operation', actor: 'operator', usefulOutcome: 'accept only actual evidence', sourceRequirement: 'native: receipt projection', reasonNow: 'finite phase', likelihood: 'required', mandatory: true, ownerTask: 'T1', productionEntry: 'application', realCollaborators: ['real Rust callback'], substitutedBoundaries: [], oracle, negativeControl: { input: 'wrong', expectedRejection: 'independent fixed truth comparison', commandRef: 'negative' }, fixtures: 'isolated scratch', evidence: 'original child process and NDJSON', limitations: ['fixture demonstrates adapter admission, not arbitrary producer honesty'], invalidationKeys: ['production.mjs', 'independent-truth.txt'], restoration: 'application child exited with no live owned process' };
  const command = (id, mode, feature, kind) => ({ id, kind, argv: [path.join(root, 'application'), '--vlbv-child', mode, path.join(root, feature.path)], cwd: root, scenarioIds: interleaved ? ['VLBV-1', 'VLBV-2'] : ['VLBV-1'] });
  const commands = modes.map((mode, i) => command(`run-${i}`, mode, features[i], 'scenario'));
  const negative = command('negative', 'fail', controlFeature, 'negativeControl');
  const contract = { contractVersion: '2', kind: 'kbd.adopted-contract', identity,
    digestNormalization: { algorithm: 'sha256', command: "jq -S -c 'del(.identity.contractDigest) | del(.identity.planRevision)'", stalenessRule: 'exact bound snapshot bytes' },
    adoptionSource: 'Phase39 isolated BDD fixture', decisionOwner: 'fixture owner', amendmentPolicy: 'existing signed canonical amendment path',
    releaseClaim: { userOperation: 'project evidence', supportedEnvironments: ['isolated fixture'], currentCapabilities: 'read-only adapter', explicitUnknowns: ['producer truth'] }, statusVocabulary: ['PASS', 'FAIL', 'BLOCKED', 'NOT_EXERCISED'],
    scenarioMatrix: [scenario], functionalScope: ['receipt projection'], resilienceScope: { policy: 'required cases never waived', deferredOptional: [] }, exclusions: ['publication'], requiredSafetyChecks: [{ id: 'isolated', description: 'scratch only', scenarioIds: ['VLBV-1'] }],
    budgets: { timeMinutes: 60, toolCalls: 100, repairAttempts: 2, noProgressLimit: 3, tokenBudget: 'UNKNOWN', tokenFallback: 'time and calls', exhaustion: 'BLOCKED' }, evidenceInvalidation: 'candidate/source/contract changes', workerRoutes: { agent: 'fixture', model: 'none' },
    boundaryPacket: { postImplementationOnly: true, singleRun: true, commands: [...commands, negative], expectedEffects: ['isolated processes'], requiredApprovals: 'fixture owner', receipts: ['process', 'oracle', 'control', 'cleanup'], cleanup: ['scratch restored'], stopConditions: ['failure'] },
    status: { implementationComplete: true }, calibration: { examplesPath: 'independent-truth.txt', sha256: truth.sha256 }, method: { name: 'VLBV', packVersion: '1.15.0', referenceSha256: truth.sha256 }, closure: { implementationComplete: true, behaviorAccepted: false, releaseEligible: false, releaseAuthorized: false } };
  if (fixture === 'missing adopted scenario' || interleaved) contract.scenarioMatrix.push({ ...scenario, id: 'VLBV-2', name: 'Absent obligation' });
  if (fixture === 'stochastic profile') { scenario.nondeterministic = true; scenario.stochastic = { samples: 2, grader: 'not supported here', stoppingRule: 'two observations' }; }
  const contractRef = put('contract.snapshot.json', contract);
  const canonicalStatus = put('canonical-status.json', { projectId: identity.projectId, runId: fixture === 'wrong canonical run' ? 'wrong-run' : identity.runId });
  const disposition = { schema: 'kbd.phase-disposition/v1', projectId: identity.projectId, runId: identity.runId, phaseId: identity.phaseId, phaseStatus: 'in_progress', contractBound: true, disposition: 'OPEN', dispatchAllowed: true, successorAllowed: false, awaitingSuccessor: false, consumptionHighWater: {}, binding: { contractPath: contractRef.path, contractDigest: fixture === 'wrong contract binding' ? '0'.repeat(64) : contractRef.sha256, scenarioDigests: {}, planRevision: 0, adoptionDecisionId: 'fixture-adoption', bindingRevision: 1 } };
  const canonicalDisposition = put('canonical-disposition.json', disposition);
  const authorization = { schemaVersion: 1, kind: 'kbd.boundary-authorization', phaseId: identity.phaseId, events: [{ boundary: 'B4', authorizedAt: new Date().toISOString(), authorizer: 'isolated fixture owner', contractDigest: contractRef.sha256 }] };
  const authorizationRef = put('boundary-authorization.json', authorization);
  const runs = [], bindings = [], assertions = [], negativeControls = [], cleanup = [];
  const second = {id: 'VLBV-2', bindings: [], assertions: [], negativeControls: [], cleanup: []};
  let packetCalls = 0;
  function execute(command, label) {
    const startedAt = new Date().toISOString();
    packetCalls += 1;
    const child = spawnSync(command.argv[0], command.argv.slice(1), { cwd: root, env, encoding: 'utf8' });
    if (child.error || child.status === null) throw new Error(`application unavailable: ${child.error ?? child.signal}`);
    const finishedAt = new Date().toISOString();
    const messages = put(`${label}.ndjson`, child.stdout);
    const stderr = put(`${label}.stderr.txt`, child.stderr);
    const process = put(`${label}.process.json`, { projectId: identity.projectId, runId: identity.runId, phase: identity.phaseId, contractDigest: contractRef.sha256, candidateDigest: candidate.sha256, commandId: command.id, argv: command.argv, cwd: root, startedAt, finishedAt, exitCode: child.status, messages, pid: child.pid, signal: child.signal, stderr });
    originals.push(put(`${label}.original.ndjson`, child.stdout), put(`${label}.original.process.json`, load(process)));
    const stream = child.stdout.trim().split('\n').map(line => JSON.parse(line));
    return { messages, process, stream };
  }
  for (let i = 0; i < modes.length; i++) {
    if (i > 0 && ['bound authorized repair', 'self repair predecessor', 'repair after unexercised cleanup', 'repair after inconsistent control'].includes(fixture)) {
      authorization.events.push({ boundary: 'B5', authorizedAt: new Date().toISOString(), authorizer: 'isolated fixture owner', contractDigest: contractRef.sha256, reason: 'bounded repair of VLBV-1' });
      Object.assign(authorizationRef, put('boundary-authorization.json', authorization));
    }
    const actual = execute(commands[i], `run-${i}`);
    const run = { id: `stream-${i}`, messages: actual.messages, process: actual.process, candidate, authorizationIndex: i > 0 ? (['bound authorized repair', 'self repair predecessor', 'repair after unexercised cleanup', 'repair after inconsistent control'].includes(fixture) ? 1 : 0) : 0, ...(i ? { predecessor: fixture === 'self repair predecessor' ? `stream-${i}` : 'stream-0' } : {}) };
    runs.push(run);
    if (fixture === 'wrong candidate') { const receipt = load(run.process); receipt.candidateDigest = '0'.repeat(64); run.process = put(run.process.path, receipt); }
    const pickle = actual.stream.find(m => m.pickle).pickle;
    bindings.push({ runId: run.id, pickleId: pickle.id });
    const otherPickle = interleaved ? actual.stream.filter(m => m.pickle)[1].pickle : null;
    if (otherPickle) second.bindings.push({runId: run.id, pickleId: otherPickle.id});
    const starts = actual.stream.filter(m => m.testCaseStarted).map(m => m.testCaseStarted);
    for (const start of starts) {
      const actualCase = actual.stream.find(m => m.testCase?.id === start.testCaseId).testCase;
      const other = actualCase.pickleId !== pickle.id;
      if (other && !interleaved) continue;
      const scenarioId = other ? second.id : scenario.id;
      const steps = actual.stream.filter(m => m.testStepFinished?.testCaseStartedId === start.id).map(m => m.testStepFinished);
      const step = fixture === 'skipped step' ? steps[1] : steps[0];
      const observed = put(`observed-${i}-${start.attempt}-${scenarioId}.json`, step);
      const assertion = { id: 'fixed-truth', scenarioId, runId: run.id, testCaseStartedId: start.id, testStepId: step.testStepId, candidateDigest: candidate.sha256, oracle,
        status: step.testStepResult.status === 'FAILED' ? 'FAIL' : step.testStepResult.status === 'PASSED' ? 'PASS' : 'NOT_EXERCISED', observed: JSON.stringify(step.testStepResult), exercised: ['PASSED', 'FAILED'].includes(step.testStepResult.status), realCollaborators: scenario.realCollaborators, substitutedBoundaries: [], evidence: [observed, truth, actual.messages] };
      if (fixture !== 'green without assertion') (other ? second.assertions : assertions).push(put(`assertion-${i}-${start.attempt}-${scenarioId}.json`, assertion));
    }
    let control;
    if (fixture !== 'missing control') {
      control = execute(negative, `control-${i}`);
      const start = control.stream.find(m => m.testCaseStarted).testCaseStarted;
      const step = control.stream.find(m => m.testStepFinished).testStepFinished;
      negativeControls.push(put(`control-${i}.json`, { scenarioId: scenario.id, runId: run.id, assertionId: 'fixed-truth', candidateDigest: candidate.sha256, commandRef: 'negative', expectedRejection: scenario.negativeControl.expectedRejection,
        observedRejection: step.testStepResult.message?.includes('independent fixed truth comparison') ? scenario.negativeControl.expectedRejection : 'unexpected failure', exercised: true, sensitive: fixture !== 'non-sensitive control', messages: control.messages, process: control.process, testCaseStartedId: start.id, testStepId: step.testStepId, evidence: [control.messages, truth] }));
    }
    if (interleaved) second.negativeControls.push(put(`control-${i}-second.json`, {...load(negativeControls.at(-1)), scenarioId: second.id}));
    const restored = put(`restored-${i}.json`, { childPid: load(actual.process).pid, exitCode: load(actual.process).exitCode, signal: load(actual.process).signal, childExited: true });
    cleanup.push(put(`cleanup-${i}.json`, { scenarioId: scenario.id, runId: run.id, candidateDigest: candidate.sha256, requirement: scenario.restoration, status: fixture === 'unknown cleanup' ? 'BLOCKED' : 'PASS', observed: 'child process exited; no owned live process', evidence: [restored, actual.process] }));
    if (interleaved) second.cleanup.push(put(`cleanup-${i}-second.json`, {...load(cleanup.at(-1)), scenarioId: second.id}));
    let changed = false;
    const stream = actual.stream;
    const finishIndex = () => stream.findIndex(m => m.testRunFinished);
    if (fixture === 'unknown step status') { stream.find(m => m.testStepFinished).testStepFinished.testStepResult.status = 'INVENTED'; changed = true; }
    if (fixture === 'truncated stream') { stream.splice(finishIndex(), 1); changed = true; }
    if (fixture === 'orphan step start') { const extra = structuredClone(stream.find(m => m.testStepStarted)); extra.testStepStarted.testCaseStartedId = 'missing-attempt'; stream.splice(finishIndex(), 0, extra); changed = true; }
    if (fixture === 'orphan global hook finish') { const extra = structuredClone(stream.find(m => m.testRunHookFinished)); extra.testRunHookFinished.testRunHookStartedId = 'missing-hook'; stream.splice(finishIndex(), 0, extra); changed = true; }
    if (fixture === 'unfinished global hook') { stream.splice(stream.findIndex(m => m.testRunHookFinished), 1); changed = true; }
    if (fixture === 'reversed step lifecycle') { const start = stream.findIndex(m => m.testStepStarted), end = stream.findIndex(m => m.testStepFinished); [stream[start], stream[end]] = [stream[end], stream[start]]; changed = true; }
    if (fixture === 'nonzero first attempt') { stream.find(m => m.testCaseStarted).testCaseStarted.attempt = 1; changed = true; }
    if (fixture === 'duplicate attempt number') { stream.filter(m => m.testCaseStarted)[1].testCaseStarted.attempt = 0; changed = true; }
    if (fixture === 'missing promised retry') { stream.find(m => m.testCaseFinished).testCaseFinished.willBeRetried = true; changed = true; }
    if (fixture === 'unpromised successor') { stream.find(m => m.testCaseFinished).testCaseFinished.willBeRetried = false; changed = true; }
    if (fixture === 'concealed parse error') { stream.splice(finishIndex(), 0, {parseError: {source: {uri: 'broken.feature'}, message: 'actual parse failure omitted by summary'}}); changed = true; }
    if (fixture === 'concealed global failure') { stream[finishIndex()].testRunFinished.success = true; changed = true; }
    if (fixture === 'concealed run exception') { stream[finishIndex()].testRunFinished.exception = {type: 'Error', message: 'unhandled global exception'}; changed = true; }
    if (interleaved) {
      const firstId = starts[0].id, secondId = starts[1].id;
      const owned = (message, id) => message.testCaseStarted?.id === id || message.testStepStarted?.testCaseStartedId === id || message.testStepFinished?.testCaseStartedId === id || message.testCaseFinished?.testCaseStartedId === id;
      const first = stream.filter(m => owned(m, firstId)), other = stream.filter(m => owned(m, secondId));
      const insertion = stream.findIndex(m => owned(m, firstId));
      const merged = [];
      for (let j = 0; j < Math.max(first.length, other.length); j++) { if (first[j]) merged.push(first[j]); if (other[j]) merged.push(other[j]); }
      const rest = stream.filter(m => !owned(m, firstId) && !owned(m, secondId));
      rest.splice(insertion, 0, ...merged); stream.splice(0, stream.length, ...rest); changed = true;
    }
    if (changed) {
      run.messages = put(`run-${i}.mutant.ndjson`, stream.map(m => JSON.stringify(m)).join('\n') + '\n');
      const process = {...load(run.process), messages: run.messages};
      if (fixture === 'concealed global failure') process.exitCode = 0;
      run.process = put(`run-${i}.mutant.process.json`, process);
    }
    if (fixture === 'substituted product stream') run.messages = control.messages;
    if (fixture === 'substituted control stream') { const receipt = load(negativeControls.at(-1)); receipt.messages = actual.messages; negativeControls[negativeControls.length - 1] = put(`control-${i}.substituted.json`, receipt); }
    if (fixture === 'nonboolean exercised claim') { const receipt = load(assertions[0]); receipt.exercised = 'false'; assertions[0] = put('assertion-invalid-boolean.json', receipt); }
    if (fixture === 'orphan failed assertion') { const receipt = {...load(assertions[0]), id: 'orphan-failure', testCaseStartedId: 'missing-attempt', status: 'FAIL'}; assertions.push(put('assertion-orphan.json', receipt)); }
    if (fixture === 'conflicting cleanup') cleanup.push(put('cleanup-conflict.json', {...load(cleanup[0]), status: 'BLOCKED', observed: 'uncertain restoration'}));
    if (fixture === 'orphan control assertion') negativeControls.push(put('control-orphan.json', {...load(negativeControls[0]), assertionId: 'missing-assertion'}));
    if (fixture === 'repair after unexercised cleanup' && i === 0) {
      assertions.length = 0; negativeControls.length = 0;
      cleanup[0] = put('cleanup-unexercised.json', {...load(cleanup[0]), status: 'BLOCKED', observed: 'restoration remains unknown'});
    }
    if (fixture === 'repair after inconsistent control' && i === 0) {
      negativeControls[0] = put('control-earlier-inconsistent.json', {...load(negativeControls[0]), messages: actual.messages});
    }
    if (fixture === 'conflicting control') negativeControls.push(put('control-conflict.json', {...load(negativeControls[0]), sensitive: false}));
  }
  if (fixture === 'green without assertion') negativeControls.length = 0;
  if (fixture === 'stale production source') fs.appendFileSync(path.join(root, source.path), '\n// changed after freeze\n');
  const bundle = { schema: 'cucumber.vlbv-receipts/v1', contract: contractRef, canonicalStatus, canonicalDisposition, authorization: authorizationRef, candidate,
    executedBy: { harness: 'Phase39 real-process BDD fixture', version: '0.23.0', model: 'none (deterministic fixture producer)' },
    budgetConsumption: { source: 'isolated receipt producer: elapsed wall time and observed child calls; fixture repair count', values: { timeMinutes: { value: (Date.now() - baseline) / 60000, class: 'measured' }, toolCalls: { value: packetCalls, class: 'measured' }, repairAttempts: { value: modes.length - 1, class: 'measured' }, noProgress: { value: 0, class: 'measured' } } },
    runs, scenarios: [{ id: scenario.id, bindings, assertions, negativeControls, cleanup }, ...(interleaved ? [second] : [])] };
  put('bundle.json', bundle);
  const sentinel = put('CANONICAL-NO-WRITE', 'untouched');
  const protectedRefs = [contractRef, canonicalStatus, canonicalDisposition, authorizationRef, candidate, sentinel, ...runs.flatMap(r => [r.messages, r.process])];
  const filesBefore = fs.readdirSync(root, { recursive: true }).sort();
  const before = protectedRefs.map(ref => digest(fs.readFileSync(path.join(root, ref.path))));
  const converted = spawnSync('node', [path.join(root, source.path), '--root', root, '--bundle', 'bundle.json'], { cwd: root, encoding: 'utf8' });
  if (!converted.stdout.trim()) throw new Error(`converter emitted no result: ${converted.stderr}`);
  const output = JSON.parse(converted.stdout);
  let migration = null;
  if (['substituted product stream', 'substituted control stream'].includes(fixture)) {
    const legacy = structuredClone(bundle);
    if (fixture === 'substituted product stream') {
      const receipt = load(legacy.runs[0].process); delete receipt.messages;
      legacy.runs[0].process = put('legacy-process.json', receipt);
    } else {
      const control = load(legacy.scenarios[0].negativeControls[0]);
      const receipt = load(control.process); delete receipt.messages;
      control.process = put('legacy-control-process.json', receipt);
      legacy.scenarios[0].negativeControls[0] = put('legacy-control.json', control);
    }
    put('legacy-bundle.json', legacy);
    const projected = spawnSync('node', [path.join(root, source.path), '--root', root, '--bundle', 'legacy-bundle.json'], {cwd: root, encoding: 'utf8'});
    migration = {exit: projected.status, output: JSON.parse(projected.stdout), stderr: projected.stderr};
    put('legacy-projection.stdout.json', projected.stdout); put('legacy-projection.stderr.txt', projected.stderr);
  }
  const unchanged = JSON.stringify(filesBefore) === JSON.stringify(fs.readdirSync(root, { recursive: true }).sort()) && before.every((value, i) => value === digest(fs.readFileSync(path.join(root, protectedRefs[i].path))));
  let canonicalReaderExit = null, canonicalReaderOutput = null;
  if (fixture === 'complete') {
    const phaseDir = `.kbd-orchestrator/phases/${identity.phaseId}`;
    put(`${phaseDir}/adopted/${contractRef.sha256}.json`, fs.readFileSync(path.join(root, contractRef.path)));
    put(`${phaseDir}/boundary-results.json`, output);
    for (const name of ['kbd-phase-close-eval.sh', 'kbd-closure-packet.mjs']) put(`reader/${name}`, fs.readFileSync(path.join(repository, 'tests/fixtures/vlbv', name)));
    const reader = spawnSync('bash', [path.join(root, 'reader/kbd-phase-close-eval.sh'), '--phase', identity.phaseId, '--status-file', path.join(root, canonicalStatus.path), '--disposition-file', path.join(root, canonicalDisposition.path), '--decision-sink', path.join(root, 'isolated-decisions.txt')], { cwd: root, encoding: 'utf8' });
    canonicalReaderExit = reader.status; canonicalReaderOutput = reader.stdout + reader.stderr;
  }
  put('projection.stdout.json', converted.stdout); put('projection.stderr.txt', converted.stderr);
  if (archive) {
    fs.mkdirSync(archive, {recursive: true});
    for (const entry of fs.readdirSync(root)) if (entry !== 'application') fs.cpSync(path.join(root, entry), path.join(archive, entry), {recursive: true});
    fs.writeFileSync(path.join(archive, 'archive-note.txt'), 'Original process/NDJSON/receipt artifacts retained. The executed application binary is not duplicated; its actual digest remains in candidate.json.\n');
  }
  process.stdout.write(JSON.stringify({ contract, bundle, output, migration, originals, archive, exit: converted.status, stderr: converted.stderr, unchanged, canonicalReaderExit, canonicalReaderOutput }));
} finally { fs.rmSync(root, { recursive: true, force: true }); }
