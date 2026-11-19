#!/usr/bin/env node
// Read-only projection. Never executes packet commands or writes canonical state.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const args = process.argv.slice(2);
const opt = name => args[args.indexOf(name) + 1];
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const equal = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const nonempty = value => typeof value === 'string' && value.trim().length > 0;
const requireThat = (condition, message) => { if (!condition) throw new Error(message); };
const array = value => { requireThat(Array.isArray(value), 'expected array'); return value; };
const combine = (left, right) => left === 'FAIL' || right === 'FAIL' ? 'FAIL' : left === 'NOT_EXERCISED' || right === 'NOT_EXERCISED' ? 'NOT_EXERCISED' : 'PASS';
const verdicts = new Set(['PASS', 'FAIL', 'BLOCKED', 'NOT_EXERCISED']);
let root;
function confined(relative) {
  requireThat(nonempty(relative) && !path.isAbsolute(relative) && !relative.split(/[\\/]/).includes('..'), 'unconfined evidence path');
  const resolved = fs.realpathSync(path.resolve(root, relative));
  requireThat(resolved.startsWith(root + path.sep) && fs.statSync(resolved).isFile(), 'evidence escapes root or is not a file');
  return resolved;
}
function bytes(ref) {
  requireThat(ref && /^[a-f0-9]{64}$/.test(ref.sha256), 'missing artifact digest');
  const data = fs.readFileSync(confined(ref.path));
  requireThat(hash(data) === ref.sha256, `stale artifact: ${ref.path}`);
  return data;
}
const read = ref => JSON.parse(bytes(ref));
function evidence(refs) { requireThat(array(refs).length > 0, 'observation evidence missing'); refs.forEach(bytes); }
function messages(ref) {
  const envelopes = bytes(ref).toString('utf8').split('\n').filter(line => line.trim()).map(line => JSON.parse(line));
  const of = kind => envelopes.filter(e => e[kind]).map(e => e[kind]);
  const positions = new Map(envelopes.flatMap((envelope, i) => Object.values(envelope).map(value => [value, i])));
  const before = (left, right) => positions.get(left) < positions.get(right);
  requireThat(of('testRunStarted').length === 1 && of('testRunFinished').length === 1 && typeof of('testRunFinished')[0].success === 'boolean', 'incomplete Messages run');
  requireThat(of('meta').length === 1 && ['34.2.1', '35.0.0'].includes(of('meta')[0]?.protocolVersion), 'unsupported Messages protocol');
  const runStart = of('testRunStarted')[0], runFinish = of('testRunFinished')[0];
  requireThat(before(runStart, runFinish), 'run lifecycle order');
  requireThat(!runFinish.testRunStartedId || runFinish.testRunStartedId === runStart.id, 'wrong run finish reference');
  requireThat(of('parseError').length === 0, 'parse error prevents acceptance');
  requireThat(!runFinish.exception, 'run exception prevents acceptance');
  const index = kind => { const result = new Map(); for (const item of of(kind)) { requireThat(nonempty(item.id) && !result.has(item.id), `duplicate ${kind} id`); result.set(item.id, item); } return result; };
  const pickles = index('pickle'), cases = index('testCase'), starts = index('testCaseStarted');
  const hookDefinitions = index('hook'), hookStarts = index('testRunHookStarted');
  for (const started of hookStarts.values()) {
    const finishes = of('testRunHookFinished').filter(f => f.testRunHookStartedId === started.id);
    requireThat(finishes.length === 1 && hookDefinitions.has(started.hookId) && nonempty(runStart.id) && started.testRunStartedId === runStart.id, 'incomplete or dangling global hook');
    const finished = finishes[0];
    requireThat(before(runStart, started) && before(started, finished) && before(finished, runFinish), 'global hook lifecycle order');
    requireThat(['PASSED', 'SKIPPED', 'FAILED', 'PENDING', 'UNDEFINED', 'AMBIGUOUS', 'UNKNOWN'].includes(finished.result?.status), 'unknown global hook status');
    requireThat(['PASSED', 'SKIPPED'].includes(finished.result.status), 'global hook failed or unexercised; unsuccessful run');
  }
  requireThat(of('testRunHookFinished').length === hookStarts.size, 'orphan global hook finish');
  const attempts = [...starts.values()].map(start => {
    const testCase = cases.get(start.testCaseId);
    requireThat(testCase && pickles.has(testCase.pickleId), 'dangling Messages case/pickle');
    requireThat(!testCase.testRunStartedId || testCase.testRunStartedId === runStart.id, 'wrong test case run reference');
    const finish = of('testCaseFinished').filter(f => f.testCaseStartedId === start.id);
    requireThat(finish.length === 1, 'incomplete or duplicate Messages attempt');
    requireThat(before(runStart, start) && before(start, finish[0]) && before(finish[0], runFinish), 'attempt lifecycle order');
    const steps = of('testStepFinished').filter(f => f.testCaseStartedId === start.id);
    requireThat(steps.every(s => ['PASSED', 'FAILED', 'SKIPPED', 'PENDING', 'UNDEFINED', 'AMBIGUOUS', 'UNKNOWN'].includes(s.testStepResult?.status)), 'unknown step status');
    const began = of('testStepStarted').filter(f => f.testCaseStartedId === start.id);
    requireThat(steps.length === testCase.testSteps.length && began.length === steps.length, 'incomplete Messages steps');
    for (const planned of testCase.testSteps) {
      const ends = steps.filter(s => s.testStepId === planned.id), beginnings = began.filter(s => s.testStepId === planned.id);
      requireThat(ends.length === 1 && beginnings.length === 1, 'duplicate or missing step receipt');
      requireThat(before(start, beginnings[0]) && before(beginnings[0], ends[0]) && before(ends[0], finish[0]), 'step lifecycle order');
    }
    return { testCaseStartedId: start.id, testCaseId: start.testCaseId, pickleId: testCase.pickleId,
      attempt: start.attempt, willBeRetried: finish[0].willBeRetried, steps, start, finish: finish[0] };
  });
  requireThat(of('testCaseFinished').length === attempts.length, 'orphan attempt finish');
  const stepCount = attempts.reduce((sum, a) => sum + a.steps.length, 0);
  requireThat(of('testStepFinished').length === stepCount, 'orphan step finish');
  requireThat(of('testStepStarted').length === stepCount, 'orphan step start');
  for (const testCase of cases.values()) {
    const chain = attempts.filter(a => a.testCaseId === testCase.id);
    chain.forEach((attempt, i) => {
      requireThat(Number.isInteger(attempt.attempt) && attempt.attempt === i && typeof attempt.willBeRetried === 'boolean' && attempt.willBeRetried === (i + 1 < chain.length), 'invalid retry chain number or promise');
      if (i > 0) requireThat(before(chain[i - 1].finish, attempt.start), 'overlapping retry chain');
    });
  }
  return { attempts, envelopes, success: runFinish.success };
}
function project(bundle) {
  const contract = read(bundle.contract), status = read(bundle.canonicalStatus), disposition = read(bundle.canonicalDisposition);
  const candidate = read(bundle.candidate), authorization = read(bundle.authorization);
  const output = { phase: contract.identity?.phaseId, projectId: contract.identity?.projectId, runId: contract.identity?.runId,
    contractDigest: bundle.contract.sha256, sourceDigests: candidate.sourceDigests ?? {}, executedBy: bundle.executedBy,
    budgetConsumption: bundle.budgetConsumption, scenarios: [], adapter: { schema: bundle.schema, originalBundle: opt('--bundle'), inputs: bundle, runs: [], unmappedAttempts: [], trust: 'Producer receipts are testimony; content hashes establish identity, not observation truth.' } };
  const global = [];
  const check = fn => { try { fn(); } catch (error) { global.push(error.message); } };
  check(() => {
    requireThat(bundle.schema === 'cucumber.vlbv-receipts/v1', 'unsupported adapter schema');
    requireThat(contract.contractVersion === '2' && contract.kind === 'kbd.adopted-contract', 'unsupported contract profile');
    requireThat(contract.status?.implementationComplete === true, 'implementation not complete in bound contract');
    requireThat(disposition.schema === 'kbd.phase-disposition/v1' && disposition.contractBound === true && disposition.binding?.contractDigest === bundle.contract.sha256, 'wrong contract binding');
    for (const value of [status, disposition]) requireThat(value.projectId === output.projectId && value.runId === output.runId, 'wrong canonical project or run');
    requireThat(disposition.phaseId === output.phase, 'wrong canonical phase');
    requireThat(authorization.kind === 'kbd.boundary-authorization' && authorization.phaseId === output.phase, 'wrong authorization record');
    for (const key of ['harness', 'version', 'model']) requireThat(nonempty(bundle.executedBy?.[key]), 'executedBy session identity missing');
    requireThat(Object.keys(output.sourceDigests).length > 0, 'source digests missing');
    for (const [file, digest] of Object.entries(output.sourceDigests)) {
      requireThat(!/\s/.test(file), 'canonical evaluator cannot read whitespace source paths');
      bytes({ path: file, sha256: digest });
    }
    for (const key of ['build', 'configuration', 'fixtures', 'references']) array(candidate[key]).forEach(bytes);
  });
  check(() => {
    const consumption = bundle.budgetConsumption;
    requireThat(nonempty(consumption?.source), 'budget source missing');
    const extensions = contract.budgets?.extensions ?? [];
    const resulting = extensions.filter(e => e.resultingCeilings).at(-1)?.resultingCeilings;
    for (const [dimension, ceilingKey] of Object.entries({ timeMinutes: 'timeMinutes', toolCalls: 'toolCalls', repairAttempts: 'repairAttempts', noProgress: 'noProgressLimit' })) {
      const value = consumption.values?.[dimension];
      requireThat(value && ['measured', 'estimate'].includes(value.class) && Number.isFinite(value.value) && value.value >= 0, `unknown budget: ${dimension}`);
      const ceiling = resulting?.[ceilingKey] ?? ((contract.budgets?.[ceilingKey] ?? 0) + (dimension === 'noProgress' ? 0 : extensions.reduce((sum, e) => sum + (e[ceilingKey] ?? 0), 0)));
      requireThat(value.value <= ceiling && value.value >= (disposition.consumptionHighWater?.[dimension] ?? 0), `budget limit/high-water: ${dimension}`);
    }
  });
  function processReceipt(ref, candidateDigest, expectedKind, streamRef) {
    const receipt = read(ref);
    requireThat(receipt.messages && nonempty(receipt.messages.path) && nonempty(receipt.messages.sha256), 'Messages artifact binding missing: record process.messages {path, sha256} when capturing original stdout');
    requireThat(receipt.messages.path === streamRef.path && receipt.messages.sha256 === streamRef.sha256, 'Messages artifact binding differs from supplied stream');
    bytes(receipt.messages);
    requireThat(receipt.phase === output.phase && receipt.projectId === output.projectId && receipt.runId === output.runId && receipt.contractDigest === output.contractDigest, 'process identity mismatch');
    requireThat(receipt.candidateDigest === candidateDigest, 'wrong candidate');
    const command = array(contract.boundaryPacket?.commands).find(c => c.id === receipt.commandId);
    requireThat(command && (expectedKind ? command.kind === expectedKind : ['run', 'scenario'].includes(command.kind)) && equal(command.argv, receipt.argv), 'entry argv differs from adopted packet');
    requireThat(nonempty(receipt.cwd) && (!command.cwd || command.cwd === receipt.cwd), 'entry cwd differs from adopted packet');
    requireThat(Number.isFinite(Date.parse(receipt.startedAt)) && Date.parse(receipt.finishedAt) >= Date.parse(receipt.startedAt), 'invalid process timestamps');
    requireThat(Number.isInteger(receipt.exitCode) && receipt.exitCode >= 0 && receipt.exitCode !== 2 && !receipt.signal, 'process exit two or unknown');
    return receipt;
  }
  const runs = new Map();
  for (const run of array(bundle.runs)) {
    requireThat(nonempty(run.id) && !runs.has(run.id), 'duplicate run identity');
    const predecessor = run.predecessor ? runs.get(run.predecessor) : null;
    const record = { ...run, attempts: [], error: null };
    runs.set(run.id, record);
    try {
      const stream = messages(run.messages);
      record.attempts = stream.attempts; record.success = stream.success;
      if (run.predecessor) requireThat(predecessor && !predecessor.error && predecessor.receipt && predecessor.authorityRoot, 'repair predecessor must be a distinct previously admitted run');
      const frozen = read(run.candidate);
      for (const key of ['build', 'configuration', 'fixtures', 'references']) array(frozen[key]).forEach(bytes);
      record.receipt = processReceipt(run.process, run.candidate.sha256, undefined, run.messages);
      const event = authorization.events?.[run.authorizationIndex];
      requireThat(event && nonempty(event.authorizer) && event.contractDigest === output.contractDigest && Date.parse(event.authorizedAt) <= Date.parse(record.receipt.startedAt), 'missing/stale boundary authorization');
      if (run.predecessor) {
        requireThat(predecessor?.receipt && event.boundary === 'B5' && Date.parse(event.authorizedAt) >= Date.parse(predecessor.receipt.finishedAt), 'repair lacks existing bound B5 authority');
        requireThat(bundle.budgetConsumption.values.repairAttempts.value >= [...runs.values()].filter(r => r.predecessor).length, 'repair budget omits attempts');
        record.authorityRoot = predecessor.authorityRoot;
      } else { requireThat(event.boundary === 'B4', 'initial run lacks B4 authority'); record.authorityRoot = run.id; }
    } catch (error) { record.error = error.message; }
  }
  output.adapter.runs = [...runs.values()];
  for (const run of runs.values()) if (run.error) global.push(`${run.id}: ${run.error}`);
  const adopted = array(contract.scenarioMatrix), supplied = array(bundle.scenarios);
  requireThat(adopted.length > 0 && new Set(adopted.map(s => s.id)).size === adopted.length, 'invalid adopted denominator');
  requireThat(new Set(supplied.map(s => s.id)).size === supplied.length, 'duplicate scenario bindings');
  const used = new Set();
  for (const scenario of adopted) {
    const binding = supplied.find(s => s.id === scenario.id);
    const result = { id: scenario.id, status: 'NOT_EXERCISED', attempted: { bindings: binding?.bindings ?? [] }, observed: { reasons: [] },
      evidence: { contract: bundle.contract, candidate: bundle.candidate, authorization: bundle.authorization, attempts: [], assertions: binding?.assertions ?? [], negativeControls: binding?.negativeControls ?? [], cleanup: binding?.cleanup ?? [] } };
    output.scenarios.push(result);
    const reasons = result.observed.reasons;
    try {
      requireThat(global.length === 0, global.join('; '));
      requireThat(!scenario.nondeterministic && !scenario.stochastic && typeof scenario.oracle === 'object' && typeof scenario.negativeControl === 'object', 'unsupported acceptance profile');
      if (!binding) { reasons.push('adopted scenario has no exercised binding'); continue; }
      array(binding.bindings);
      const assertions = array(binding.assertions).map(ref => ({ ...read(ref), original: ref }));
      const controls = array(binding.negativeControls).map(ref => ({ ...read(ref), original: ref }));
      const cleanups = array(binding.cleanup).map(ref => ({ ...read(ref), original: ref }));
      const unique = (receipts, key, label) => {
        const seen = new Set();
        for (const receipt of receipts) { const identity = key(receipt); requireThat(!seen.has(identity), `duplicate ${label} receipt`); seen.add(identity); }
      };
      unique(controls, c => `${c.runId}:${c.assertionId}`, 'control');
      unique(cleanups, c => c.runId, 'cleanup');
      for (const assertion of assertions) {
        requireThat(typeof assertion.exercised === 'boolean', 'assertion exercised boolean is required');
        const mapped = binding.bindings.find(b => b.runId === assertion.runId && runs.get(b.runId)?.attempts.some(a => a.pickleId === b.pickleId && a.testCaseStartedId === assertion.testCaseStartedId && a.steps.some(step => step.testStepId === assertion.testStepId)));
        requireThat(assertion.scenarioId === scenario.id && mapped, 'orphan assertion receipt');
      }
      for (const receipt of [...controls, ...cleanups]) requireThat(receipt.scenarioId === scenario.id && binding.bindings.some(b => b.runId === receipt.runId), 'orphan control or cleanup receipt');
      // Supplied evidence must remain consistent even when an earlier product
      // assertion failed or was unexercised and a later authorized repair passes.
      for (const cleanup of cleanups) {
        const run = runs.get(cleanup.runId);
        requireThat(run && cleanup.candidateDigest === run.candidate.sha256 && cleanup.requirement === scenario.restoration && cleanup.status === 'PASS' && nonempty(cleanup.observed), 'cleanup unknown or incomplete');
        evidence(cleanup.evidence);
      }
      for (const control of controls) {
        const run = runs.get(control.runId);
        requireThat(assertions.some(a => a.runId === control.runId && a.id === control.assertionId), 'orphan control assertion receipt');
        requireThat(run && control.candidateDigest === run.candidate.sha256 && control.commandRef === scenario.negativeControl.commandRef && control.expectedRejection === scenario.negativeControl.expectedRejection && typeof control.exercised === 'boolean' && typeof control.sensitive === 'boolean', 'negative-control identity/observation mismatch');
        const controlProcess = processReceipt(control.process, run.candidate.sha256, 'negativeControl', control.messages);
        requireThat(controlProcess.commandId === control.commandRef, 'wrong control command'); evidence(control.evidence);
        const observed = messages(control.messages).attempts.find(a => a.testCaseStartedId === control.testCaseStartedId)?.steps.find(s => s.testStepId === control.testStepId);
        requireThat(observed, 'control observation does not resolve to a recorded step');
        if (control.exercised && control.sensitive) {
          requireThat(control.observedRejection === control.expectedRejection, 'negative-control identity/observation mismatch');
          requireThat(observed.testStepResult.status === 'FAILED', 'control did not exercise a rejecting assertion');
        }
      }
      if (!binding.bindings.length) { reasons.push('adopted scenario has no exercised binding'); continue; }
      const perRun = [];
      const recordRun = (run, verdict) => {
        const existing = perRun.find(item => item.run.id === run.id);
        if (existing) existing.verdict = combine(existing.verdict, verdict);
        else perRun.push({ run, verdict });
      };
      for (const mapped of binding.bindings) {
        const run = runs.get(mapped.runId);
        requireThat(run && !run.error, run?.error ?? 'unknown run binding');
        const attempts = run.attempts.filter(a => a.pickleId === mapped.pickleId);
        if (!attempts.length) { recordRun(run, 'NOT_EXERCISED'); reasons.push('no started attempt for adopted binding'); continue; }
        let runVerdict = 'PASS';
        for (const attempt of attempts) {
          const key = `${run.id}:${attempt.testCaseStartedId}`;
          requireThat(!used.has(key), 'attempt assigned to more than one adopted scenario'); used.add(key);
          const item = { ...attempt, runId: run.id, candidate: run.candidate, process: run.process, messages: run.messages, authorizationIndex: run.authorizationIndex, status: 'NOT_EXERCISED', assertions: [] };
          result.evidence.attempts.push(item);
          const own = assertions.filter(a => a.runId === run.id && a.testCaseStartedId === attempt.testCaseStartedId);
          if (!own.length) {
            reasons.push('required path or oracle not exercised'); if (runVerdict !== 'FAIL') runVerdict = 'NOT_EXERCISED'; continue;
          }
          let attemptVerdict = 'PASS';
          for (const assertion of own) {
            requireThat(assertion.scenarioId === scenario.id && assertion.candidateDigest === run.candidate.sha256 && nonempty(assertion.id) && equal(assertion.oracle, scenario.oracle), 'assertion identity/oracle mismatch');
            requireThat(equal(assertion.realCollaborators, scenario.realCollaborators) && equal(assertion.substitutedBoundaries, scenario.substitutedBoundaries), 'real/substituted boundary mismatch');
            const step = attempt.steps.find(s => s.testStepId === assertion.testStepId);
            requireThat(step && verdicts.has(assertion.status) && nonempty(assertion.observed), 'invalid assertion receipt'); evidence(assertion.evidence);
            item.assertions.push(assertion);
            if (!assertion.exercised || assertion.status === 'NOT_EXERCISED') { attemptVerdict = combine(attemptVerdict, 'NOT_EXERCISED'); continue; }
            requireThat(assertion.status !== 'BLOCKED', 'oracle receipt blocked');
            if (assertion.status === 'FAIL') { requireThat(step.testStepResult.status === 'FAILED', 'claimed product failure was not observed by the bound assertion'); attemptVerdict = 'FAIL'; continue; }
            requireThat(step.testStepResult.status === 'PASSED', 'claimed passing assertion did not pass');
            const control = controls.find(c => c.runId === run.id && c.assertionId === assertion.id);
            if (!control || control.exercised !== true || control.sensitive !== true) { attemptVerdict = combine(attemptVerdict, 'NOT_EXERCISED'); reasons.push('required negative control absent or non-sensitive'); continue; }
          }
          if (attemptVerdict === 'PASS' && attempt.steps.some(s => ['SKIPPED', 'PENDING', 'UNDEFINED', 'AMBIGUOUS', 'UNKNOWN'].includes(s.testStepResult?.status))) attemptVerdict = 'NOT_EXERCISED';
          if (attemptVerdict === 'PASS') requireThat(attempt.steps.every(s => s.testStepResult.status === 'PASSED'), 'unclassified failing step');
          const cleanup = cleanups.find(c => c.runId === run.id);
          requireThat(cleanup && cleanup.scenarioId === scenario.id && cleanup.candidateDigest === run.candidate.sha256 && cleanup.requirement === scenario.restoration && cleanup.status === 'PASS' && nonempty(cleanup.observed), 'cleanup unknown or incomplete'); evidence(cleanup.evidence);
          item.status = attemptVerdict;
          if (attemptVerdict === 'FAIL' || runVerdict === 'FAIL') runVerdict = 'FAIL'; else if (attemptVerdict !== 'PASS') runVerdict = attemptVerdict;
        }
        recordRun(run, runVerdict);
      }
      let selected;
      for (const current of perRun) {
        if (selected) requireThat(current.run.predecessor === selected.run.id, 'later run cannot erase evidence without explicit repair lineage');
        selected = current;
      }
      requireThat(selected, 'empty attempt selection');
      requireThat(selected.run.candidate.sha256 === bundle.candidate.sha256, 'final evidence is not current candidate');
      if (selected.verdict === 'PASS') {
        requireThat(selected.run.success === true, 'unsuccessful run cannot support passing acceptance');
        requireThat(selected.run.receipt.exitCode === 0, 'nonzero product process cannot support passing acceptance');
      }
      result.status = selected.verdict;
      if (result.status === 'PASS') reasons.push('current exercised oracle, sensitivity and restoration receipts agree');
      else if (result.status === 'FAIL') reasons.push('exercised product oracle unmet; all failures retained');
    } catch (error) { result.status = 'BLOCKED'; reasons.push(error.message); }
  }
  for (const run of runs.values()) for (const attempt of run.attempts) if (!used.has(`${run.id}:${attempt.testCaseStartedId}`)) output.adapter.unmappedAttempts.push({ runId: run.id, messages: run.messages, ...attempt });
  if (output.adapter.unmappedAttempts.some(a => a.steps.some(s => s.testStepResult.status !== 'PASSED'))) {
    for (const result of output.scenarios) {
      if (result.status !== 'FAIL') result.status = 'BLOCKED';
      result.observed.reasons.push('unmapped failing attempt cannot be classified as accepted scope');
    }
  }
  return output;
}
try {
  requireThat(args.length === 4 && args.includes('--root') && args.includes('--bundle'), 'usage: --root <directory> --bundle <relative.json>');
  root = fs.realpathSync(opt('--root'));
  const output = project(JSON.parse(fs.readFileSync(confined(opt('--bundle')), 'utf8')));
  process.stdout.write(`${JSON.stringify(output, null, 2)}\n`);
  process.exitCode = output.scenarios.some(s => s.status === 'BLOCKED' || s.status === 'NOT_EXERCISED') ? 2 : output.scenarios.some(s => s.status === 'FAIL') ? 3 : 0;
} catch (error) { process.stderr.write(`BLOCKED: ${error.message}\n`); process.exitCode = 2; }
