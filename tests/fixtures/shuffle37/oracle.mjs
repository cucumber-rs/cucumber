// Phase37 acceptance oracle. Execute only after the implementation phase ends.
// The package sources and licenses beside this file are unchanged npm artifacts.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const request = JSON.parse(readFileSync(0, 'utf8'));
if (!Array.isArray(request.items) || typeof request.seed !== 'string') {
  throw new Error('Expected {items: array, seed: string}; implicit/random seeds are outside this oracle');
}
const packages = new Map([
  ['knuth-shuffle-seeded', 'knuth-shuffle-seeded-1.0.6/index.js'],
  ['seed-random', 'seed-random-2.2.0/index.js'],
]);
const cache = new Map();
function load(name) {
  if (cache.has(name)) return cache.get(name).exports;
  const relative = packages.get(name);
  if (!relative) throw new Error(`Unpinned oracle dependency: ${name}`);
  const filename = fileURLToPath(new URL(relative, import.meta.url));
  const module = { exports: {} };
  cache.set(name, module);
  const invoke = compileFunction(readFileSync(filename, 'utf8'), ['module', 'exports', 'require'], { filename });
  invoke(module, module.exports, load);
  return module.exports;
}
const shuffle = load('knuth-shuffle-seeded');
process.stdout.write(JSON.stringify(shuffle(Array.from(request.items), request.seed)));
