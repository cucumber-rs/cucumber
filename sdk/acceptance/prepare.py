#!/usr/bin/env python3
"""Phase46 boundary preparation. Never invoke during implementation.

Builds and installs real native packages in disposable external consumer roots.
Original logs/receipts are exclusive-created; no retry overwrites earlier proof.
The focused BDD cases own imports, runtime assertions and TypeScript compilation.
"""
from pathlib import Path
import datetime
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = Path(os.environ.get('SDK46_EVIDENCE', ROOT / 'docs/compatibility/evidence/sdk46-first')).resolve()
ALLOWED_PREPARATION = {'source-before-boundary.json', 'repair-reason.md', 'packet.txt'}
EVIDENCE.mkdir(parents=True, exist_ok=True)
if any(p.name not in ALLOWED_PREPARATION for p in EVIDENCE.iterdir()):
    raise SystemExit('Refusing to overwrite previous SDK46 evidence; choose a fresh authorized packet directory')
with (EVIDENCE / '.run-claimed').open('x', encoding='utf-8') as claim:
    claim.write(datetime.datetime.now(datetime.timezone.utc).isoformat())
ARTIFACTS = EVIDENCE / 'artifacts'
ARTIFACTS.mkdir()
(EVIDENCE / '.gitignore').write_text('/artifacts/\n', encoding='utf-8')
CONSUMERS = Path(tempfile.mkdtemp(prefix='cucumber-sdk46-consumers-')).resolve()
(CONSUMERS / '.sdk46-owned.json').write_text(json.dumps({'kind': 'sdk46-disposable-consumers', 'root': str(ROOT), 'evidence': str(EVIDENCE)}) + '\n')
ENV = os.environ.copy()
for key in ('NODE_PATH', 'NODE_OPTIONS', 'PYTHONPATH', 'PYTHONHOME', 'VIRTUAL_ENV', 'CARGO_TARGET_DIR', 'RUSTFLAGS', 'RUSTDOCFLAGS'):
    ENV.pop(key, None)
ENV.update(RUSTUP_TOOLCHAIN='1.88.0', PYTHONNOUSERSITE='1', PIP_DISABLE_PIP_VERSION_CHECK='1')
COMMANDS = []


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def record(name, value):
    with (EVIDENCE / name).open('x', encoding='utf-8') as output:
        json.dump(value, output, indent=2)
        output.write('\n')


def run(argv, cwd=ROOT, extra_env=None):
    number = len(COMMANDS) + 1
    prefix = f'prepare-{number:02d}'
    env = dict(ENV)
    env.update(extra_env or {})
    entry = {'argv': [str(a) for a in argv], 'cwd': str(cwd), 'startedAt': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    with (EVIDENCE / (prefix + '.stdout')).open('xb') as out, (EVIDENCE / (prefix + '.stderr')).open('xb') as err:
        try:
            result = subprocess.run(entry['argv'], cwd=cwd, env=env, stdout=out, stderr=err)
            entry['exitCode'] = result.returncode
        except OSError as exc:
            entry['spawnError'] = str(exc)
            entry['exitCode'] = None
    entry['finishedAt'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    entry['stdout'] = prefix + '.stdout'
    entry['stderr'] = prefix + '.stderr'
    COMMANDS.append(entry)
    record(prefix + '.json', entry)
    print(f'{prefix}: exit={entry["exitCode"]} {entry["argv"][0]}', flush=True)
    if entry['exitCode'] != 0:
        raise RuntimeError(f'{prefix} failed; original output retained')
    return (EVIDENCE / (prefix + '.stdout')).read_text(encoding='utf-8')


def snapshot():
    files = {}
    for base in ('sdk', 'bindings', 'src', 'vendor/gherkin/src', '.github/workflows'):
        for path in sorted((ROOT / base).rglob('*')):
            if not path.is_file() or any(part in ('target', 'node_modules', '__pycache__', '.venv') for part in path.parts):
                continue
            if path.name in ('runtime.json',) or path.suffix in ('.node', '.whl', '.tgz'):
                continue
            files[str(path.relative_to(ROOT))] = sha(path)
    return files


try:
    record('preparation-start.json', {'consumerRoot': str(CONSUMERS), 'root': str(ROOT), 'platform': platform.platform(), 'architecture': platform.machine(), 'pythonExecutable': sys.executable, 'source': snapshot()})
    if sys.platform != 'darwin' or platform.machine() != 'arm64':
        raise RuntimeError('This first adopted package proof is Darwin arm64 only')
    versions = {}
    for label, argv in (('rustc', ['rustc', '--version']), ('cargo', ['cargo', '--version']), ('node', ['node', '--version']), ('npm', ['npm', '--version']), ('python', [sys.executable, '--version'])):
        versions[label] = run(argv).strip()
    if not versions['rustc'].startswith('rustc 1.88.0 ') or versions['node'] != 'v26.5.0' or versions['python'] != 'Python 3.11.3':
        raise RuntimeError('Adopted runtime identity mismatch: expected Rust1.88.0, Node26.5.0 and Python3.11.3; see original version receipts')
    for manifest in ('sdk/Cargo.toml', 'bindings/node/Cargo.toml', 'bindings/python/Cargo.toml'):
        run(['cargo', 'generate-lockfile', '--manifest-path', str(ROOT / manifest)])
    target = CONSUMERS / 'cargo-target'
    rust_env = {'CARGO_TARGET_DIR': str(target)}
    run(['cargo', 'build', '--release', '--locked', '--manifest-path', str(ROOT / 'bindings/node/Cargo.toml')], extra_env=rust_env)
    native_node = ROOT / 'bindings/node/cucumber_sdk.node'
    shutil.copy2(target / 'release/libcucumber_sdk_node.dylib', native_node)
    packed = json.loads(run(['npm', 'pack', '--json', '--ignore-scripts', '--pack-destination', str(ARTIFACTS)], ROOT / 'bindings/node'))
    archive = ARTIFACTS / packed[0]['filename']
    node_root = CONSUMERS / 'node'
    node_root.mkdir()
    (node_root / 'package.json').write_text(json.dumps({'name': 'sdk46-external-consumer', 'private': True, 'version': '0.0.0', 'dependencies': {'cucumber-native-local': str(archive), 'typescript': '6.0.3'}}) + '\n')
    run(['npm', 'install', '--ignore-scripts', '--no-audit', '--no-fund'], node_root)
    for name in ('node-consumer.cjs', 'consumer.ts'):
        shutil.copy2(ROOT / 'sdk/acceptance' / name, node_root / name)
    shutil.copy2(node_root / 'package-lock.json', EVIDENCE / 'node-consumer-package-lock.json')

    builder = CONSUMERS / 'builder'
    python_env = CONSUMERS / 'python'
    run([sys.executable, '-m', 'venv', str(builder)])
    run([sys.executable, '-m', 'venv', str(python_env)])
    builder_python = builder / 'bin/python'
    python_executable = python_env / 'bin/python'
    run([builder_python, '-m', 'pip', 'install', 'maturin==1.12.6', 'jsonschema==4.25.1'])
    run([builder_python, '-m', 'maturin', 'build', '--release', '--locked', '--manifest-path', str(ROOT / 'bindings/python/Cargo.toml'), '--interpreter', str(python_executable), '--out', str(ARTIFACTS)], ROOT / 'bindings/python', rust_env)
    wheels = list(ARTIFACTS.glob('*.whl'))
    if len(wheels) != 1:
        raise RuntimeError('Expected exactly one native wheel')
    run([python_executable, '-m', 'pip', 'install', '--no-deps', str(wheels[0])])
    shutil.copy2(ROOT / 'sdk/acceptance/python-consumer.py', CONSUMERS / 'python-consumer.py')
    run([builder_python, '-m', 'pip', 'freeze'])
    run([python_executable, '-m', 'pip', 'freeze'])

    rust_root = CONSUMERS / 'rust'
    (rust_root / 'src').mkdir(parents=True)
    manifest = '''[package]
name = "sdk46-external-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
publish = false
[workspace]
[dependencies]
cucumber-sdk = { path = SDK_PATH }
cucumber = { path = ROOT_PATH, features = ["macros", "output-messages"] }
futures = "0.3"
regex = "1"
serde_json = "1"
tempfile = "3"
'''.replace('SDK_PATH', json.dumps(str(ROOT / 'sdk'))).replace('ROOT_PATH', json.dumps(str(ROOT)))
    (rust_root / 'Cargo.toml').write_text(manifest)
    shutil.copy2(ROOT / 'sdk/acceptance/consumer.rs', rust_root / 'src/main.rs')
    run(['cargo', 'generate-lockfile', '--manifest-path', str(rust_root / 'Cargo.toml')])
    run(['cargo', 'build', '--release', '--locked', '--manifest-path', str(rust_root / 'Cargo.toml')], rust_root, rust_env)
    rust_executable = target / 'release/sdk46-external-consumer'
    shutil.copy2(rust_root / 'Cargo.toml', EVIDENCE / 'rust-consumer-Cargo.toml')
    shutil.copy2(rust_root / 'Cargo.lock', EVIDENCE / 'rust-consumer-Cargo.lock')
    metadata = json.loads(run(['cargo', 'metadata', '--locked', '--format-version', '1', '--manifest-path', str(rust_root / 'Cargo.toml')], rust_root, rust_env))
    record('rust-dependency-provenance.json', [{'name': p['name'], 'version': p['version'], 'source': p['source'], 'manifestPath': p['manifest_path'], 'license': p.get('license')} for p in metadata['packages']])

    oracles = {}
    for kind, source in (('jsOracleRoot', 'tests/interop/rerun'), ('gherkinOracleRoot', 'tests/interop/gherkin43')):
        destination = CONSUMERS / kind
        shutil.copytree(ROOT / source, destination, ignore=shutil.ignore_patterns('node_modules', 'target'))
        run(['npm', 'ci', '--ignore-scripts', '--no-audit', '--no-fund'], destination)
        oracles[kind] = str(destination)
    node_executable = shutil.which('node')
    runtime = {'root': str(ROOT), 'evidence': str(EVIDENCE), 'consumerRoot': str(CONSUMERS), 'nodeRoot': str(node_root), 'nodeExecutable': node_executable, 'nodeConsumer': str(node_root / 'node-consumer.cjs'), 'pythonExecutable': str(python_executable), 'pythonConsumer': str(CONSUMERS / 'python-consumer.py'), 'acceptancePython': str(builder_python), 'rustExecutable': str(rust_executable), 'typescriptExecutable': str(node_root / 'node_modules/typescript/bin/tsc'), **oracles}
    runtime['artifacts'] = [{'path': str(p), 'sha256': sha(p), 'bytes': p.stat().st_size} for p in (archive, wheels[0], native_node, rust_executable)]
    for p in python_env.rglob('*.so'):
        runtime['artifacts'].append({'path': str(p), 'sha256': sha(p), 'bytes': p.stat().st_size})
    record('runtime.json', runtime)
    (ROOT / 'sdk/acceptance/runtime.json').write_text(json.dumps(runtime, indent=2) + '\n')
    record('preparation-complete.json', {'status': 'PREPARED', 'commands': COMMANDS, 'sourceAfterResolution': snapshot(), 'note': 'No behavior acceptance claimed by preparation. Cleanup occurs after both BDD packets; paths remain until then.'})
except BaseException as exc:
    record('preparation-failure.json', {'error': str(exc), 'commands': COMMANDS, 'consumerRoot': str(CONSUMERS), 'cleanup': 'Consumer root retained for investigation; not silently deleted or reused.'})
    raise
