#!/usr/bin/env node
// kbd-closure-packet.mjs — derived closure view for a KBD value phase (VLBV CR-21).
//
// Written by kbd-phase-close-eval.sh for every outcome (CLOSED or refused). It is
// a readable *view* generated from the bound contract snapshot and
// boundary-results.json; the canonical disposition recorded through
// `prometheus kbd phase close` remains the authority.
//
// Usage: node kbd-closure-packet.mjs --snapshot <contract.json> --evidence <boundary-results.json>
//          --out-dir <phase-dir> --disposition <CLOSED|REFUSED_BLOCKED|REFUSED_FAIL> --reason <text>
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const args = process.argv.slice(2);
const opt = name => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const readJson = file => {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return null;
  }
};
const contract = readJson(opt('--snapshot')) ?? {};
const evidencePath = opt('--evidence');
const evidence = readJson(evidencePath) ?? {};
const outDir = opt('--out-dir');
const disposition = opt('--disposition') ?? 'UNKNOWN';
const reason = opt('--reason') ?? '';
if (!outDir) {
  process.stderr.write('kbd-closure-packet: --out-dir is required\n');
  process.exit(1);
}

const scenarios = contract.scenarioMatrix ?? [];
const entries = evidence.scenarios ?? [];
const verdictOf = id => {
  const own = entries.filter(e => e.id === id || String(e.id).startsWith(`${id}-`));
  if (!own.length) return 'NOT_EXERCISED';
  if (own.some(e => e.status === 'FAIL')) return 'FAIL';
  if (own.every(e => e.status === 'PASS')) return 'PASS';
  return own.some(e => e.status === 'BLOCKED') ? 'BLOCKED' : 'NOT_EXERCISED';
};
const rows = scenarios.map(s => ({
  id: s.id,
  name: s.name,
  actor: s.actor ?? null,
  usefulOutcome: s.usefulOutcome ?? null,
  mandatory: s.mandatory ?? true,
  verdict: verdictOf(s.id),
  limitations: s.limitations ?? [],
  substitutedBoundaries: s.substitutedBoundaries ?? [],
  realCollaborators: s.realCollaborators ?? [],
}));
const mandatory = rows.filter(r => r.mandatory !== false);
const methodContext = readJson(path.join(outDir, 'method-context.json'));

// VLBV CR-33 passive scan (report only; never a guard and never verdict-changing):
// runner output under the contract's declared runnerOutputGlobs whose mtime is
// earlier than the recorded B4 authorization event.
function findProjectRoot(start) {
  let dir = path.resolve(start);
  while (dir !== path.dirname(dir) && !fs.existsSync(path.join(dir, '.kbd-orchestrator'))) dir = path.dirname(dir);
  return dir;
}
function globRegex(glob) {
  let out = '';
  for (let i = 0; i < glob.length; i += 1) {
    const ch = glob[i];
    if (ch === '*' && glob[i + 1] === '*') {
      if (glob[i + 2] === '/') { out += '(?:.*/)?'; i += 2; } else { out += '.*'; i += 1; }
    } else if (ch === '*') out += '[^/]*';
    else if (ch === '?') out += '[^/]';
    else out += ch.replace(/[.+^${}()|[\]\\]/g, '\\$&');
  }
  return new RegExp(`^${out}$`);
}
function walkFiles(root, base) {
  const start = path.join(root, base);
  if (!fs.existsSync(start)) return [];
  const found = [];
  const walk = dir => {
    for (const name of fs.readdirSync(dir)) {
      if (name === '.git' || name === 'node_modules') continue;
      const full = path.join(dir, name);
      const stat = fs.lstatSync(full);
      if (stat.isDirectory()) walk(full);
      else if (stat.isFile()) found.push({ rel: path.relative(root, full).split(path.sep).join('/'), mtimeMs: stat.mtimeMs });
    }
  };
  if (fs.lstatSync(start).isFile()) {
    found.push({ rel: base, mtimeMs: fs.lstatSync(start).mtimeMs });
    return found;
  }
  walk(start);
  return found;
}
function preAuthorizationScan() {
  const globs = contract.boundaryPacket?.runnerOutputGlobs ?? [];
  const authorization = readJson(path.join(outDir, 'boundary-authorization.json'));
  const event = (authorization?.events ?? []).find(e => e.boundary === 'B4');
  const authorizedAt = event?.authorizedAt ?? null;
  if (!globs.length) return { status: 'no-globs', authorizedAt, globs, scanned: 0, artifacts: [] };
  if (!authorizedAt || Number.isNaN(Date.parse(authorizedAt))) return { status: 'no-authorization', authorizedAt: null, globs, scanned: 0, artifacts: [] };
  const projectRoot = path.resolve(opt('--project-root') ?? findProjectRoot(outDir));
  const authMs = Date.parse(authorizedAt);
  const seen = new Map();
  for (const glob of globs) {
    const regex = globRegex(glob);
    const base = glob.split('/').filter((part, i, parts) => !/[*?]/.test(parts.slice(0, i + 1).join('/'))).join('/');
    for (const file of walkFiles(projectRoot, base)) if (regex.test(file.rel)) seen.set(file.rel, file.mtimeMs);
  }
  const artifacts = [...seen.entries()].filter(([, mtimeMs]) => mtimeMs < authMs).sort(([a], [b]) => a.localeCompare(b))
    .map(([file, mtimeMs]) => ({ path: file, mtime: new Date(mtimeMs).toISOString() }));
  return { status: artifacts.length ? 'found' : 'none', authorizedAt, globs, scanned: seen.size, artifacts };
}
const preAuthorization = preAuthorizationScan();
const statusComplete =
  contract.closure?.implementationComplete ?? contract.status?.implementationComplete ?? false;
const behaviorAccepted = disposition === 'CLOSED' && mandatory.every(r => r.verdict === 'PASS');
const closure = {
  schemaVersion: 1,
  kind: 'kbd.closure-view',
  generatedAt: new Date().toISOString(),
  authority:
    'derived view; the canonical disposition recorded by prometheus kbd phase close is authoritative',
  phaseId: contract.identity?.phaseId ?? evidence.phase ?? null,
  contractVersion: contract.contractVersion ?? null,
  contractDigest: evidence.contractDigest ?? null,
  evidenceDigest:
    evidencePath && fs.existsSync(evidencePath)
      ? crypto.createHash('sha256').update(fs.readFileSync(evidencePath)).digest('hex')
      : null,
  disposition,
  reason,
  states: {
    implementationComplete: Boolean(statusComplete),
    behaviorAccepted,
    releaseEligible: behaviorAccepted && rows.every(r => r.verdict === 'PASS'),
    releaseAuthorized: Boolean(contract.closure?.releaseAuthorized) && behaviorAccepted,
  },
  method: methodContext
    ? {
        name: methodContext.selectedMethod,
        source: methodContext.methodSource ?? null,
        deviation: methodContext.deviation ?? null,
      }
    : { name: contract.method?.name ?? 'VLBV', source: 'contract', deviation: null },
  executedBy: evidence.executedBy ?? null,
  preAuthorizationArtifacts: preAuthorization,
  scenarios: rows,
};
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, 'closure.json'), JSON.stringify(closure, null, 2) + '\n');

const list = (items, empty = 'none') =>
  items.length ? items.map(i => `- ${i}`).join('\n') : `- ${empty}`;
const works = rows.filter(r => r.verdict === 'PASS');
const remains = rows.filter(r => r.verdict !== 'PASS');
const claim = contract.releaseClaim ?? {};
const md = `# Closure packet — ${closure.phaseId ?? 'phase'}

Disposition: **${disposition}**${reason ? ` (${reason})` : ''}. This packet is a derived view of
the bound contract (${closure.contractDigest ? `sha256:${closure.contractDigest}` : 'digest unknown'}) and
\`boundary-results.json\`${closure.evidenceDigest ? ` (sha256:${closure.evidenceDigest})` : ''}; the canonical KBD disposition is authoritative.

## What works, and for whom

${list(
  works.map(r => `${r.id} — ${r.usefulOutcome ?? r.name}${r.actor ? ` (for ${r.actor})` : ''}`),
  'no scenario passed'
)}

## Under which conditions

${list([
  ...(claim.supportedEnvironments ?? []).map(e => `environment: ${e}`),
  ...rows.flatMap(r => r.realCollaborators.map(c => `${r.id} real collaborator: ${c}`)),
  ...rows.flatMap(r =>
    r.substitutedBoundaries.map(b => `${r.id} SUBSTITUTED boundary (not a real claim): ${b}`)
  ),
])}

## Limits

${list([...rows.flatMap(r => r.limitations.map(l => `${r.id}: ${l}`)), ...(claim.explicitUnknowns ?? []).map(u => `unknown: ${u}`)])}

## What remains

${list(
  remains.map(r => `${r.id} — ${r.verdict}${r.mandatory === false ? ' (not mandatory)' : ''}`),
  'nothing: every scenario passed'
)}

## Pre-authorization runner artifacts

${(() => {
  const p = preAuthorization;
  if (p.status === 'no-globs') return '- no runnerOutputGlobs declared in the contract; scan not possible';
  if (p.status === 'no-authorization') return '- no B4 authorization event recorded (boundary-authorization.json); scan not possible';
  if (p.status === 'none') return `- none found (${p.scanned} runner artifact(s) under ${p.globs.join(', ')}, all modified at or after the B4 authorization at ${p.authorizedAt})`;
  return p.artifacts.map(a => `- ${a.path} (modified ${a.mtime}, before the B4 authorization at ${p.authorizedAt})`).join('\n');
})()}

This scan is passive evidence (VLBV 04, 10): it reports and never blocks.

## Closure states

| State | Value |
|---|---|
| Implementation complete | ${closure.states.implementationComplete} |
| Behavior accepted | ${closure.states.behaviorAccepted} |
| Release eligible | ${closure.states.releaseEligible} |
| Release authorized | ${closure.states.releaseAuthorized} (a separate owner decision) |

Method: ${closure.method.name}${closure.method.deviation ? ` — **deviation** to ${closure.method.deviation.method}: cadence "${closure.method.deviation.cadence}", reason "${closure.method.deviation.reason}" (decision ${closure.method.deviation.decisionId})` : ''}.
Executed by: ${closure.executedBy ? `${closure.executedBy.harness ?? '?'} ${closure.executedBy.version ?? ''} (${closure.executedBy.model ?? 'model unknown'})` : 'not recorded in boundary-results.json'}.
`;
fs.writeFileSync(path.join(outDir, 'closure-packet.md'), md);
