// Acceptance support only. No external modules are imported before preparation.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

const repository = fs.realpathSync(fileURLToPath(new URL('../../', import.meta.url)));
const boundaries = ['interop', 'interop/expressions', 'interop/gherkin43', 'interop/rerun'];
const hash = value => createHash('sha256').update(value).digest('hex');
const fileHash = file => hash(fs.readFileSync(file));
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));
const writeJson = (file, value) => fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n');
const sameScripts = (left, right) => left && Object.keys(left).length === Object.keys(right).length
  && Object.entries(right).every(([name, digest]) => left[name] === digest);
const requireCondition = (condition, message) => {
  if (!condition) throw new Error(message);
};

function identity(directory, relative) {
  const scripts = Object.fromEntries(fs.readdirSync(directory).filter(name => name.endsWith('.mjs'))
    .sort().map(name => [name, fileHash(path.join(directory, name))]));
  return { relative, packageJsonSha256: fileHash(path.join(directory, 'package.json')),
    packageLockSha256: fileHash(path.join(directory, 'package-lock.json')), scripts };
}

function validate(root, expected) {
  const marker = readJson(path.join(root, 'oracle-prepared.json'));
  requireCondition(marker.schemaVersion === 1 && marker.root === fs.realpathSync(root),
    'Oracle preparation root/schema does not match');
  const entry = marker.boundaries.find(item => item.relative === expected.relative);
  requireCondition(entry && entry.install?.exitCode === 0 && Array.isArray(entry.install.argv),
    `Missing successful locked preparation for ${expected.relative}`);
  for (const key of ['packageJsonSha256', 'packageLockSha256']) {
    requireCondition(entry[key] === expected[key],
      `Stale oracle preparation ${expected.relative}: ${key}`);
  }
  requireCondition(sameScripts(entry.scripts, expected.scripts),
    `Stale oracle preparation ${expected.relative}: scripts`);
  const directory = path.join(root, expected.relative);
  const actual = identity(directory, expected.relative);
  requireCondition(JSON.stringify(actual) === JSON.stringify(expected),
    `Mirrored oracle source/lock differs for ${expected.relative}`);
  const manifest = readJson(path.join(directory, 'package.json'));
  const packages = Object.entries(manifest.dependencies).map(([name, version]) => {
    const packagePath = path.join(directory, 'node_modules', name, 'package.json');
    const resolved = fs.realpathSync(packagePath);
    const within = path.relative(fs.realpathSync(directory), resolved);
    requireCondition(!within.startsWith(`..${path.sep}`) && !path.isAbsolute(within),
      `Installed ${name} escapes its owned oracle root`);
    const installed = readJson(resolved);
    requireCondition(installed.version === version, `Wrong installed ${name}: expected ${version}`);
    return { name, version, packagePath: resolved };
  });
  return { directory, packages };
}

// Invoke npm's JavaScript entry with the exact Node executable. This also avoids
// executing Windows .cmd shims through a shell or interpreting paths as code.
function npmCli() {
  const nodeDirectory = path.dirname(process.execPath);
  const candidates = [
    path.join(nodeDirectory, 'node_modules/npm/bin/npm-cli.js'),
    path.join(nodeDirectory, '../lib/node_modules/npm/bin/npm-cli.js'),
    ...String(process.env.PATH || '').split(path.delimiter).filter(Boolean).flatMap(directory => [
      path.join(directory, 'npm'), path.join(directory, 'node_modules/npm/bin/npm-cli.js'),
    ]),
  ];
  for (const candidate of candidates) {
    try {
      const resolved = fs.realpathSync(candidate);
      if (path.basename(resolved) === 'npm-cli.js' && fs.statSync(resolved).isFile()) return resolved;
    } catch { /* Try the next conventional npm installation location. */ }
  }
  throw new Error('npm-cli.js is required to prepare the locked official acceptance collaborators');
}

async function prepare(source, expected) {
  const root = path.join(repository, 'target', 'acceptance-oracles', hash(JSON.stringify(expected)));
  fs.mkdirSync(root, { recursive: true });
  const lock = path.join(root, '.preparing');
  const deadline = Date.now() + 300_000;
  while (true) {
    try { fs.mkdirSync(lock); break; }
    catch (error) {
      if (error.code !== 'EEXIST') throw error;
      requireCondition(Date.now() < deadline, `Timed out waiting for oracle preparation: ${lock}`);
      await delay(100);
    }
  }
  try {
    if (fs.existsSync(path.join(root, 'oracle-prepared.json'))) {
      validate(root, expected);
      return root;
    }
    const directory = path.join(root, expected.relative);
    fs.mkdirSync(directory, { recursive: true });
    for (const name of ['package.json', 'package-lock.json', ...Object.keys(expected.scripts)]) {
      fs.copyFileSync(path.join(source, name), path.join(directory, name));
    }
    const cli = npmCli();
    const version = spawnSync(process.execPath, [cli, '--version'], { encoding: 'utf8', timeout: 30_000 });
    requireCondition(!version.error && version.status === 0, 'Cannot identify the npm preparation executable');
    const argv = [cli, 'ci', '--ignore-scripts', '--no-audit', '--no-fund', '--prefix', directory];
    const result = spawnSync(process.execPath, argv, { encoding: 'utf8', timeout: 180_000,
      maxBuffer: 16 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] });
    fs.writeFileSync(path.join(root, 'npm-ci.stdout.log'), result.stdout || '');
    fs.writeFileSync(path.join(root, 'npm-ci.stderr.log'), result.stderr || '');
    const install = { argv: [process.execPath, ...argv], exitCode: result.status,
      npm: { entry: cli, version: version.stdout.trim() },
      node: { executable: fs.realpathSync(process.execPath), version: process.version },
      error: result.error?.message || null };
    writeJson(path.join(root, 'npm-ci.json'), install);
    requireCondition(!result.error && result.status === 0,
      `Locked npm ci failed for ${expected.relative}; see ${root}/npm-ci.stderr.log: ${result.error?.message || result.stderr}`);
    writeJson(path.join(root, 'oracle-prepared.json'), {
      schemaVersion: 1, root: fs.realpathSync(root), boundaries: [{ ...expected, install }],
    });
    validate(root, expected);
    return root;
  } finally { fs.rmdirSync(lock); }
}

try {
  requireCondition(Boolean(process.argv[2]), 'Expected an official acceptance script');
  const sourceScript = fs.realpathSync(process.argv[2]);
  const source = path.dirname(sourceScript);
  const relative = boundaries.find(name => fs.realpathSync(path.join(repository, 'tests', name)) === source);
  requireCondition(relative && path.extname(sourceScript) === '.mjs',
    'Oracle script must belong directly to one of the four retained package boundaries');
  const expected = identity(source, relative);
  const explicit = process.env.CUCUMBER_ORACLE_ROOT;
  const root = explicit === undefined ? await prepare(source, expected) : fs.realpathSync(explicit);
  const { directory, packages } = validate(root, expected);
  const script = path.join(directory, path.basename(sourceScript));
  const argv = [script, ...process.argv.slice(3)];
  const started = new Date().toISOString();
  const output = spawnSync(process.execPath, argv, { stdio: ['inherit', 'pipe', 'pipe'],
    maxBuffer: 64 * 1024 * 1024, timeout: 180_000 });
  const receipts = path.join(root, 'executions');
  fs.mkdirSync(receipts, { recursive: true });
  const receipt = path.join(receipts, `${process.pid}-${Date.now()}`);
  fs.writeFileSync(`${receipt}.stdout`, output.stdout || Buffer.alloc(0));
  fs.writeFileSync(`${receipt}.stderr`, output.stderr || Buffer.alloc(0));
  writeJson(`${receipt}.json`, { schemaVersion: 1, boundary: relative, script: path.basename(script),
    scriptSha256: expected.scripts[path.basename(script)], packageLockSha256: expected.packageLockSha256,
    root: fs.realpathSync(root), packages, node: { executable: fs.realpathSync(process.execPath), version: process.version },
    argv: [process.execPath, ...argv], cwd: process.cwd(), started, finished: new Date().toISOString(),
    exitCode: output.status, signal: output.signal, error: output.error?.message || null,
    stdoutSha256: hash(output.stdout || ''), stderrSha256: hash(output.stderr || '') });
  if (output.stdout) process.stdout.write(output.stdout);
  if (output.stderr) process.stderr.write(output.stderr);
  if (output.error) throw output.error;
  process.exitCode = output.status ?? 1;
} catch (error) {
  console.error(`Official acceptance oracle failed: ${error.message}`);
  process.exitCode = 1;
}
