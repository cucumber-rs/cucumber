"""Phase-46 behavior driver. Invoked by Gherkin steps, never as a unit suite.

The expected source graphs are official upstream artifacts, and the rerun oracle
is the pinned, actual JavaScript formatter/collector. Native outputs never create
expected graphs. Every process and control retains its original evidence.
"""
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import traceback
import jsonschema

RUNTIME = json.loads(Path(sys.argv[1]).read_text())
CASE, MODE = sys.argv[2:4]
ROOT = Path(RUNTIME['root']).resolve()
CONSUMER = Path(RUNTIME['consumerRoot']).resolve()
EVIDENCE = Path(RUNTIME['evidence']) / (CASE + '-' + MODE)
EVIDENCE.mkdir()  # Refuse reuse rather than truncate a previous execution.
PHASE = ROOT / '.kbd-orchestrator/phases/phase-46-native-sdk'
SCHEMA = json.loads((PHASE / 'sdk-contract.schema.json').read_text())
PREDECESSOR_IDENTITIES = PHASE / 'oracle-identities.json'
IDENTITIES = json.loads(PREDECESSOR_IDENTITIES.read_text())['files']
SUCCESSOR_IDENTITIES = RUNTIME.get('successorSourceIdentities')
if SUCCESSOR_IDENTITIES is not None:
    # Only the explicitly adopted successor may change local harness provenance.
    # Official fixture/package/license expectations remain the predecessor bytes.
    map_path = Path(SUCCESSOR_IDENTITIES['path'])
    map_bytes = map_path.read_bytes()
    if hashlib.sha256(map_bytes).hexdigest() != SUCCESSOR_IDENTITIES['sha256']:
        raise AssertionError('successor identity map digest differs from frozen phase source')
    successor = json.loads(map_bytes)
    allowed = {'tests/messages_corpus.rs', 'tests/messages_markdown.rs', 'tests/messages_rerun.rs'}
    if successor.get('schemaVersion') != 1 or successor.get('phaseId') != 'phase-47-contribution-readiness':
        raise AssertionError('unknown successor source identity contract')
    if successor.get('predecessorSha256') != hashlib.sha256(PREDECESSOR_IDENTITIES.read_bytes()).hexdigest():
        raise AssertionError('predecessor oracle identity artifact changed')
    if set(successor.get('files', {})) != allowed or not allowed.issubset(IDENTITIES):
        raise AssertionError('successor override must contain exactly the three adopted local harness sources')
    for name, expected in successor['files'].items():
        if not isinstance(expected, str) or len(expected) != 64 or any(ch not in '0123456789abcdef' for ch in expected):
            raise AssertionError('invalid frozen successor source digest')
        IDENTITIES[name] = expected

OBSERVATIONS = []
COUNT = 0


def demand(condition, message):
    if not condition:
        raise AssertionError(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def identities():
    actual = {name: digest(ROOT / name) for name in IDENTITIES}
    demand(actual == IDENTITIES, 'pinned oracle identity changed')
    return actual


def dump(name, value):
    (EVIDENCE / name).write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def process(argv, payload='', cwd=None, allowed=(0,), env=None):
    global COUNT
    COUNT += 1
    label = f'{COUNT:04d}'
    child_env = os.environ.copy()
    for name in ('PYTHONPATH', 'PYTHONHOME', 'VIRTUAL_ENV', 'NODE_PATH', 'NODE_OPTIONS'):
        child_env.pop(name, None)
    child_env['PYTHONNOUSERSITE'] = '1'
    if env:
        child_env.update(env)
    result = subprocess.run([str(v) for v in argv], input=payload, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            cwd=cwd or CONSUMER, env=child_env, timeout=240)
    dump(label + '-process.json', {'argv': [str(v) for v in argv], 'cwd': str(cwd or CONSUMER),
                                  'exit': result.returncode, 'stdinSha256': hashlib.sha256(payload.encode()).hexdigest()})
    (EVIDENCE / (label + '-stdin.txt')).write_text(payload)
    (EVIDENCE / (label + '-stdout.txt')).write_text(result.stdout)
    (EVIDENCE / (label + '-stderr.txt')).write_text(result.stderr)
    demand(result.returncode in allowed, f'child {label} exit {result.returncode}: {result.stderr}')
    return result


def schema(value, definition):
    jsonschema.Draft202012Validator({'$schema': SCHEMA['$schema'], '$defs': SCHEMA['$defs'],
                                    '$ref': '#/$defs/' + definition}).validate(value)


def request(operation, **args):
    value = {'version': 1, 'operation': operation, 'input': args}
    schema(value, 'request')
    return json.dumps(value, ensure_ascii=False)


def compile_request(source, uri='sdk.feature', format='gherkin', language='en'):
    return request('compile', source=source, uri=uri, format=format, defaultLanguage=language)


def rerun_request(envelopes, separator='\n'):
    ndjson = '\n'.join(json.dumps(e, ensure_ascii=False) for e in envelopes) + '\n'
    return request('rerun', ndjson=ndjson, separator=separator)


def hosts(requests, facade=False):
    commands = {
        'rust': [RUNTIME['rustExecutable']],
        'node': [RUNTIME['nodeExecutable'], RUNTIME['nodeConsumer']],
        'python': [RUNTIME['pythonExecutable'], '-I', RUNTIME['pythonConsumer']],
    }
    result = {}
    for host, argv in commands.items():
        output = json.loads(process(argv, json.dumps({'requests': requests, 'facade': facade}),
                           cwd=RUNTIME['nodeRoot'] if host == 'node' else None).stdout)
        demand(len(output['responses']) == len(requests), host + ' lost a request')
        for response in output['responses']:
            schema(response, 'response')
        for key in ('modulePath', 'nativePath', 'executable', 'manifestDir'):
            if key in output:
                path = Path(output[key]).resolve()
                demand(not path.is_relative_to(ROOT), host + ' resolved inside checkout')
                demand(path.is_relative_to(CONSUMER), host + ' resolved outside isolated consumer root')
        if host != 'rust':
            demand(Path(output['nativePath']).is_file(), host + ' missing native artifact')
        result[host] = output
    return result


def success(response):
    demand(response['ok'] is True, 'expected success: ' + json.dumps(response))
    return response['result']


def error(response, code, uri=None):
    demand(response['ok'] is False, 'invalid input produced successful result')
    demand(response['error']['code'] == code, f'expected {code}: {response}')
    if uri is not None:
        demand(response['error'].get('uri') == uri, 'original URI lost in error')


def graph(result):
    return {key: result[key] for key in ('gherkinDocument', 'pickles')}


def equal_graph(expected, actual):
    """Only declared IDs are renamed, bijectively. No URI/field normalization."""
    mapping, expected_ids = {}, set()
    def collect(e, a):
        demand(type(e) is type(a), 'graph value type differs')
        if isinstance(e, dict):
            demand(e.keys() == a.keys(), 'graph field set differs')
            if 'id' in e:
                demand(a['id'] not in mapping and e['id'] not in expected_ids, 'duplicate graph ID')
                mapping[a['id']] = e['id']
                expected_ids.add(e['id'])
            for key in e:
                collect(e[key], a[key])
        elif isinstance(e, list):
            demand(len(e) == len(a), 'graph array length differs')
            for left, right in zip(e, a):
                collect(left, right)
    collect(expected, actual)
    def normalize(value):
        if isinstance(value, dict):
            normalized = {}
            for key, item in value.items():
                if key == 'id' or key.endswith('Id'):
                    demand(item in mapping, 'dangling graph reference')
                    normalized[key] = mapping[item]
                elif key.endswith('Ids'):
                    demand(all(i in mapping for i in item), 'dangling graph references')
                    normalized[key] = [mapping[i] for i in item]
                else:
                    normalized[key] = normalize(item)
            return normalized
        if isinstance(value, list):
            return [normalize(item) for item in value]
        return value
    demand(expected == normalize(actual), 'full source graph differs from independent oracle')


def reject(action, label):
    try:
        action()
    except (AssertionError, jsonschema.ValidationError) as exc:
        OBSERVATIONS.append({'control': label, 'rejected': str(exc)})
    else:
        raise AssertionError('negative control was falsely accepted: ' + label)


def fixtures(kind='gherkin43-corpus', suffix='*.feature'):
    output = []
    for path in sorted((ROOT / 'tests/fixtures' / kind / 'good').glob(suffix)):
        ast = [json.loads(line) for line in Path(str(path) + '.ast.ndjson').read_text().splitlines()]
        pickles = [json.loads(line)['pickle'] for line in Path(str(path) + '.pickles.ndjson').read_text().splitlines()]
        expected = {'gherkinDocument': ast[0]['gherkinDocument'], 'pickles': pickles}
        output.append((path, expected))
    return output


def compile_fixtures(items, format='gherkin'):
    requests = [compile_request(path.read_bytes().decode(), expected['gherkinDocument']['uri'], format) for path, expected in items]
    outputs = hosts(requests)
    for host, output in outputs.items():
        for (path, expected), response in zip(items, output['responses']):
            result = success(response)
            demand(result['uri'] == expected['gherkinDocument']['uri'], 'exact source URI changed')
            demand(result['format'] == format, 'source format changed')
            equal_graph(expected, graph(result))
    OBSERVATIONS.append({'officialGoodFixtures': len(items), 'format': format, 'hosts': list(outputs)})
    return outputs


def sdk001(control):
    good = fixtures()
    bad = sorted((ROOT / 'tests/fixtures/gherkin43-corpus/bad').glob('*.feature'))
    demand(len(good) + len(bad) == 62, 'official plaintext corpus is incomplete')
    outputs = compile_fixtures(good)
    bad_outputs = hosts([compile_request(p.read_bytes().decode(), '../testdata/bad/' + p.name) for p in bad])
    for output in bad_outputs.values():
        for path, response in zip(bad, output['responses']):
            error(response, 'SOURCE_ERROR', '../testdata/bad/' + path.name)
    OBSERVATIONS.append({'officialBadFixtures': len(bad)})
    if control:
        index = next(i for i,(path,_) in enumerate(good) if path.name == 'minimal.feature')
        expected = copy.deepcopy(good[index][1])
        actual = graph(success(outputs['node']['responses'][index]))
        expected['gherkinDocument']['feature']['location']['line'] += 1
        reject(lambda: equal_graph(expected, actual), 'changed original source location')
        expected = good[index][1]
        corrupted = copy.deepcopy(actual)
        corrupted['pickles'][0]['astNodeIds'][0] = 'missing-original-id'
        reject(lambda: equal_graph(expected, corrupted), 'dangling original AST reference')


def markdown_oracle(source, uri):
    result = process([RUNTIME['nodeExecutable'], Path(RUNTIME['gherkinOracleRoot']) / 'markdown-oracle.mjs', uri, 'en'], source, cwd=RUNTIME['gherkinOracleRoot'])
    return json.loads(result.stdout)


def sdk002(control):
    compile_fixtures(fixtures('gherkin43-markdown', '*.feature.md'), 'markdown')
    sources = [
        '# Feature: Literal\n## Scenario: First\n  - Given render `@literal`\n',
        '# Feature: Literal\n## Scenario: Both\n* Given payload\n  | `@table` | café 😀 |\n  ````application/`@media`\n  literal `@body`\n  ````\n',
        '# Feature: Literal\n## Scenario: Both\n* Given payload\n```text\nbody\n```\n  | `@table` |\n',
        '`@feature`\n# Feature: Tagged\n`@rule`\n## Rule: Rule\n`@scenario`\n### Scenario Outline: Row <key>\n* Given value <key>\n`@examples`\n#### Examples:\n  | key |\n  | `@literal` |\n',
        '# Feature: Tagged\n## Scenario: ignored `@scenario`\n## Scenario: actual\n* Given action\n',
    ]
    expected = [markdown_oracle(source, f'markdown-{i}.feature.md') for i, source in enumerate(sources)]
    outputs = hosts([compile_request(source, f'markdown-{i}.feature.md', 'markdown') for i, source in enumerate(sources)])
    for output in outputs.values():
        for truth, response in zip(expected, output['responses']):
            equal_graph(truth, graph(success(response)))
    OBSERVATIONS.append({'livePinnedMarkdownBoundaries': len(sources)})
    if control:
        invalid = '# Feature: Invalid\n## Scenario: Work\n* Given payload\n```\none\n```\n```\ntwo\n```\n'
        official = process([RUNTIME['nodeExecutable'], Path(RUNTIME['gherkinOracleRoot']) / 'markdown-oracle.mjs', 'invalid.feature.md'], invalid, cwd=RUNTIME['gherkinOracleRoot'], allowed=(1,))
        demand('parseError' in official.stderr, 'official parser did not reject repeated argument')
        for output in hosts([compile_request(invalid, 'invalid.feature.md', 'markdown')]).values():
            error(output['responses'][0], 'SOURCE_ERROR', 'invalid.feature.md')
        OBSERVATIONS.append({'control': 'repeated Markdown argument', 'typedRejection': True})


def sdk003(control):
    items = [(p,e) for p,e in fixtures() if 'feature' not in e['gherkinDocument']]
    demand(items, 'official featureless expectations missing')
    outputs = compile_fixtures(items)
    for output in outputs.values():
        for response in output['responses']:
            demand(success(response)['pickles'] == [], 'invented empty-source pickle')
    if control:
        actual = graph(success(outputs['python']['responses'][0]))
        mutated = copy.deepcopy(items[0][1])
        mutated['pickles'] = [{'id': 'invented'}]
        reject(lambda: equal_graph(mutated, actual), 'invented featureless pickle')


def sdk004(control):
    invalid = ROOT / 'tests/fixtures/gherkin43-corpus/bad/not_gherkin.feature'
    valid, expected = next((p,e) for p,e in fixtures() if p.name == 'minimal.feature')
    uri = 'memory://original:42/不正.feature'
    outputs = hosts([compile_request(invalid.read_text(), uri), compile_request(valid.read_text(), expected['gherkinDocument']['uri'])])
    for output in outputs.values():
        error(output['responses'][0], 'SOURCE_ERROR', uri)
        equal_graph(expected, graph(success(output['responses'][1])))
    OBSERVATIONS.append({'sameHostRecovery': ['rust','node','python'], 'originalUri': uri})
    if control:
        fake = {'version':1,'ok':True,'result':{'kind':'compiled','uri':uri,'format':'gherkin','gherkinDocument':{},'pickles':[]}}
        reject(lambda: error(fake, 'SOURCE_ERROR', uri), 'malformed input disguised as empty success')


def sdk005(control):
    outputs = hosts([request('capabilities')])
    for output in outputs.values():
        result = success(output['responses'][0])
        demand(result['operations'] == ['capabilities','compile','rerun'] and result['foreignStepExecution'] is False, 'premature execution capability')
    OBSERVATIONS.append({'capabilities': success(outputs['rust']['responses'][0])})
    # An actual JSON object must never become a numeric protocol version merely
    # because its key matches serde_json's arbitrary-precision implementation.
    forged_version = '{"version":{"$serde_json::private::Number":"1"},"operation":"capabilities","input":{}}'
    for output in hosts([forged_version, request('capabilities')]).values():
        error(output['responses'][0], 'INVALID_REQUEST')
        success(output['responses'][1])
    OBSERVATIONS.append({'forgedNumericVersionRejected':'INVALID_REQUEST','sameHostRecovery':True})
    if control:
        raw = ['{', '[]', '{"version":2,"operation":"capabilities","input":{}}',
               '{"version":1,"operation":"execute","input":{}}',
               '{"version":1,"operation":"compile","input":{}}',
               '{"version":2,"operation":"execute","input":{}}',
               '{"version":1,"operation":"capabilities","input":{},"extra":1}']
        expected = ['INVALID_REQUEST','INVALID_REQUEST','UNSUPPORTED_VERSION','UNKNOWN_OPERATION','INVALID_REQUEST','UNSUPPORTED_VERSION','INVALID_REQUEST']
        for output in hosts(raw).values():
            for response, code in zip(output['responses'], expected):
                error(response, code)
        OBSERVATIONS.append({'control': 'strict validation precedence', 'codes': expected})


def real_messages():
    directory = EVIDENCE / 'actual-runner'
    output = json.loads(process([RUNTIME['rustExecutable'], '--messages', directory]).stdout)
    return {key: [json.loads(line) for line in Path(value['messages']).read_text().splitlines()] for key,value in output.items()}


def js_oracle(envelopes, separator='\n'):
    return json.loads(process([RUNTIME['nodeExecutable'], Path(RUNTIME['jsOracleRoot']) / 'oracle.mjs'],
                              json.dumps({'envelopes':envelopes,'separator':separator}), cwd=RUNTIME['jsOracleRoot']).stdout)


def original_attempts(envelopes, official=None):
    """Resolve original identities/locations from Messages, not native SDK output."""
    by_kind = {}
    for envelope in envelopes:
        for kind, value in envelope.items():
            by_kind.setdefault(kind, []).append(value)
    cases = {v['id']:v for v in by_kind.get('testCase',[])}
    pickles = {v['id']:v for v in by_kind.get('pickle',[])}
    locations = {}
    def visit(v):
        if isinstance(v, dict):
            if 'id' in v and 'location' in v:
                locations[v['id']] = v['location']['line']
            for child in v.values(): visit(child)
        elif isinstance(v, list):
            for child in v: visit(child)
    visit(by_kind.get('gherkinDocument',[]))
    finishes = {v['testCaseStartedId']:v for v in by_kind.get('testCaseFinished',[])}
    results = {}
    ranks = ['UNKNOWN','PASSED','SKIPPED','PENDING','UNDEFINED','AMBIGUOUS','FAILED']
    for value in by_kind.get('testStepFinished',[]):
        results.setdefault(value['testCaseStartedId'],[]).append(value['testStepResult']['status'])
    attempts = []
    for start in by_kind.get('testCaseStarted',[]):
        case = cases[start['testCaseId']]
        pickle = pickles[case['pickleId']]
        finish = finishes.get(start['id'])
        status = max(results.get(start['id'], ['UNKNOWN']), key=ranks.index) if finish else 'UNKNOWN'
        attempts.append({'startedId':start['id'],'testCaseId':case['id'],'pickleId':pickle['id'],
                         'attempt':str(start['attempt']),'uri':pickle['uri'],
                         'line':str(locations[pickle['astNodeIds'][-1]]), 'status':status,
                         'completed':finish is not None,'willBeRetried':finish.get('willBeRetried',False) if finish else False})
    if official is not None:
        projection = [{'attempt':int(a['attempt']),'status':a['status'],'willBeRetried':a['willBeRetried'],'pickleId':a['pickleId']} for a in attempts]
        demand(projection == official['attempts'], 'independent original-attempt decoding disagrees with actual JS collector')
    return attempts


def compare_plan(result, envelopes, official=None):
    starts = [v['testRunStarted'] for v in envelopes if 'testRunStarted' in v]
    finishes = [v['testRunFinished'] for v in envelopes if 'testRunFinished' in v]
    demand(result['runId'] == starts[0].get('id'), 'run identity changed')
    demand(result['runSuccess'] is finishes[-1]['success'], 'run success conflated with selected cases')
    attempts = original_attempts(envelopes, official)
    demand(result['attempts'] == attempts, 'attempt evidence lost or altered')
    entries = {}
    for attempt in attempts:
        if attempt['status'] != 'PASSED' and not attempt['willBeRetried']:
            entries.setdefault(attempt['uri'], []).append(attempt['line'])
    demand(result['entries'] == [{'uri':uri,'lines':lines} for uri,lines in entries.items()], 'original selectors changed')
    if official is not None:
        demand(result['formatted'] == official['formatted'], 'actual JS rerun formatter differs')


def sdk006(control):
    streams = real_messages()
    outputs_by_stream = {}
    for name in ('retries','outline'):
        envelopes = streams[name]
        official = js_oracle(envelopes)
        outputs = hosts([rerun_request(envelopes)])
        for output in outputs.values():
            compare_plan(success(output['responses'][0]), envelopes, official)
        outputs_by_stream[name] = outputs
        OBSERVATIONS.append({'realRunner':name,'attempts':len(official['attempts']),'formatter':official['formatted']})
    demand(len(original_attempts(streams['retries'])) == 4, 'actual retries not exercised')
    demand(sorted(a['line'] for a in original_attempts(streams['outline'])) == ['7','8','9'], 'filtered duplicate Examples rows not exercised')
    # Raw decimal literal survives the JS JSON boundary without Number conversion.
    precision = copy.deepcopy(streams['passing'])
    next(e['testCaseStarted'] for e in precision if 'testCaseStarted' in e)['attempt'] = 9007199254740993
    for output in hosts([rerun_request(precision)]).values():
        result = success(output['responses'][0])
        compare_plan(result, precision)
        demand(result['attempts'][0]['attempt'] == '9007199254740993', 'large integer rounded across FFI')
    no_cases = [copy.deepcopy(next(e for e in streams['passing'] if 'testRunStarted' in e)),
                copy.deepcopy(next(e for e in streams['passing'] if 'testRunFinished' in e))]
    no_cases[-1]['testRunFinished']['success'] = False
    for output in hosts([rerun_request(no_cases)]).values():
        result = success(output['responses'][0]); compare_plan(result, no_cases)
        demand(result['entries'] == [] and result['runSuccess'] is False, 'failed run without cases lost')
    statuses = ['UNKNOWN','PASSED','SKIPPED','PENDING','UNDEFINED','AMBIGUOUS','FAILED']
    for status in statuses:
        evidence = copy.deepcopy(streams['passing'])
        for envelope in evidence:
            if 'testStepFinished' in envelope:
                envelope['testStepFinished']['testStepResult']['status'] = status
            if 'testRunFinished' in envelope:
                envelope['testRunFinished']['success'] = status == 'PASSED'
        official = js_oracle(evidence)
        for output in hosts([rerun_request(evidence)]).values():
            compare_plan(success(output['responses'][0]), evidence, official)
    legacy = copy.deepcopy(streams['passing'])
    for envelope in legacy:
        if 'testRunStarted' in envelope: envelope['testRunStarted'].pop('id', None)
        for value in envelope.values():
            if isinstance(value, dict): value.pop('testRunStartedId', None)
    for output in hosts([rerun_request(legacy)]).values():
        result = success(output['responses'][0]); compare_plan(result, legacy)
        demand(result['runId'] is None, 'absent original run ID invented')
    OBSERVATIONS.append({'exactAttemptDecimal':'9007199254740993','failedRunWithoutSelectedCases':True,
                         'supplementalProtocolStatuses':statuses,'optionalOriginalRunId':None})
    if control:
        original = success(outputs_by_stream['outline']['node']['responses'][0])
        altered = copy.deepcopy(original); altered['attempts'].pop()
        reject(lambda: compare_plan(altered, streams['outline']), 'dropped original failed attempt')
        altered = copy.deepcopy(original); altered['attempts'][0]['line'] = '999'
        reject(lambda: compare_plan(altered, streams['outline']), 'synthetic outline row')


def sdk007(control):
    original = real_messages()['passing']
    # Use the real runner stream, changing only the numeric attempt into a raw
    # JSON map. The ordinary and >2**53 numeric cases remain covered separately.
    forged_attempt = copy.deepcopy(original)
    next(e['testCaseStarted'] for e in forged_attempt if 'testCaseStarted' in e)['attempt'] = {'$serde_json::private::Number':'1'}
    for output in hosts([rerun_request(forged_attempt), rerun_request(original)]).values():
        error(output['responses'][0], 'INVALID_MESSAGES')
        compare_plan(success(output['responses'][1]), original)
    OBSERVATIONS.append({'forgedNumericAttemptRejected':'INVALID_MESSAGES','sameHostRecovery':True})
    truncated = [e for e in original if 'testRunFinished' not in e]
    missing = copy.deepcopy(original)
    next(e['testCaseStarted'] for e in missing if 'testCaseStarted' in e)['testCaseId'] = 'missing-case'
    duplicate_finish = original + [copy.deepcopy(original[-1])]
    outputs = hosts([rerun_request(truncated),rerun_request(missing),rerun_request(duplicate_finish),request('rerun',ndjson='not-json\n',separator='\n')])
    for output in outputs.values():
        for response, code in zip(output['responses'], ['INCOMPLETE_RUN','MISSING_REFERENCE','RUN_ALREADY_FINISHED','INVALID_MESSAGES']):
            error(response, code)
    missing_variants = []
    for kind in ('pickle','document','astNode'):
        evidence = copy.deepcopy(original)
        if kind == 'pickle':
            next(e['testCase'] for e in evidence if 'testCase' in e)['pickleId'] = 'missing-pickle'
        elif kind == 'document':
            evidence = [e for e in evidence if 'gherkinDocument' not in e]
        else:
            next(e['pickle'] for e in evidence if 'pickle' in e)['astNodeIds'] = ['missing-ast-node']
        missing_variants.append(evidence)
    for output in hosts([rerun_request(e) for e in missing_variants]).values():
        for response in output['responses']: error(response, 'MISSING_REFERENCE')
    interrupted = [e for e in original if not any(k in e for k in ('testStepFinished','testCaseFinished'))]
    for output in hosts([rerun_request(interrupted)]).values():
        result = success(output['responses'][0]); compare_plan(result, interrupted)
        demand(all(not a['completed'] and a['status'] == 'UNKNOWN' for a in result['attempts']), 'interrupted attempts hidden')
    OBSERVATIONS.append({'typedErrors':['INCOMPLETE_RUN','MISSING_REFERENCE','RUN_ALREADY_FINISHED','INVALID_MESSAGES'],'interruptedAttemptsRetained':True})
    if control:
        fake = {'version':1,'ok':True,'result':{'kind':'rerun','runId':None,'runSuccess':True,'attempts':[],'entries':[],'formatted':''}}
        reject(lambda:error(fake,'INCOMPLETE_RUN'), 'truncated stream disguised as empty passing plan')


def sdk008(control):
    envelopes = real_messages()['passing']; official = js_oracle(envelopes)
    outputs = hosts([rerun_request(envelopes)])
    for output in outputs.values():
        result = success(output['responses'][0]); compare_plan(result,envelopes,official)
        demand(result['entries'] == [] and result['formatted'] == '', 'passing run selected sources')
    selection = json.loads(process([RUNTIME['rustExecutable'],'--selection'], '\n'.join(json.dumps(e) for e in envelopes)).stdout)
    demand(selection == {'selectionIsNone':True,'resolverCalls':0}, 'empty local selection invoked source resolver')
    OBSERVATIONS.append({'passingRunNoSelection':True,'attempts':len(official['attempts']),'localSelection':selection})
    if control:
        altered = copy.deepcopy(success(outputs['rust']['responses'][0])); altered['entries'] = [{'uri':'*','lines':['1']}]
        reject(lambda:compare_plan(altered,envelopes), 'empty plan replaced with all-source selection')


def sdk009(control):
    output = json.loads(process([RUNTIME['rustExecutable'],'--extension']).stdout)
    demand(output['result'] == {'owner':'external-consumer','input':{'value':7},'transformed':21}, 'consumer extension did not execute')
    demand(output['duplicateRejected'] and output['reservedRejected'] == [True,True,True], 'builtin/duplicate replacement admitted')
    schema(output['capabilities'],'response')
    demand(success(output['capabilities'])['operations'] == ['capabilities','compile','rerun'], 'extension replaced core capability')
    OBSERVATIONS.append({'externalRustExtension':output})
    if control:
        demand(output['duplicateRejected'] is True and all(output['reservedRejected']), 'reserved names not rejected')
        OBSERVATIONS.append({'control':'real duplicate/reserved registrations','rejected':True})


def sdk010(control):
    outputs = hosts([request('capabilities')])
    facade = hosts([request('capabilities'), compile_request('Feature: Facade\n Scenario: works\n  Given native\n','facade.feature'), compile_request('not gherkin','facade:error.feature')], facade=True)
    for output in facade.values():
        success(output['responses'][0]); success(output['responses'][1]); error(output['responses'][2], 'SOURCE_ERROR', 'facade:error.feature')
    artifacts = []
    for host, output in outputs.items():
        path = output.get('nativePath',output.get('executable'))
        artifacts.append({'host':host,'realpath':str(Path(path).resolve()),'sha256':digest(path)})
    source = Path(RUNTIME['nodeRoot']) / 'consumer.ts'
    shutil.copy2(ROOT / 'sdk/acceptance/consumer.ts', source)
    process([RUNTIME['nodeExecutable'], RUNTIME['typescriptExecutable'], '--strict','--target','ES2022','--module','NodeNext','--moduleResolution','NodeNext','--outDir','compiled','--traceResolution',source], cwd=RUNTIME['nodeRoot'])
    ts = json.loads(process([RUNTIME['nodeExecutable'], Path(RUNTIME['nodeRoot'])/'compiled/consumer.js'], cwd=RUNTIME['nodeRoot']).stdout)
    demand(ts['kind'] == 'compiled' and ts['uri'] == 'typescript.feature', 'compiled TS consumer did not execute public API')
    demand(ts['pickles'][0]['name'] == 'works', 'TS native compilation wrong')
    OBSERVATIONS.append({'nativeArtifacts':artifacts,'realTypeScriptConsumer':True})
    if control:
        # Remove artifacts only from disposable copies; fresh interpreters prevent cache hits.
        for host in ('node','python'):
            output = outputs[host]
            disposable = Path(tempfile.mkdtemp(prefix='sdk46-missing-' + host + '-', dir=CONSUMER))
            try:
                if host == 'node':
                    installed = Path(RUNTIME['nodeRoot'])/'node_modules/cucumber-native-local'
                    target = disposable/'node_modules/cucumber-native-local'
                    shutil.copytree(installed,target)
                    copied = target/Path(output['nativePath']).resolve().relative_to(installed.resolve())
                    copied.unlink()
                    shutil.copy2(ROOT/'sdk/acceptance/node-consumer.cjs',disposable/'consumer.cjs')
                    result = process([RUNTIME['nodeExecutable'],disposable/'consumer.cjs'],json.dumps({'requests':[request('capabilities')]}),cwd=disposable,allowed=(1,))
                else:
                    installed = Path(output['modulePath']).parent
                    target = disposable/'cucumber_native_local'
                    shutil.copytree(installed,target)
                    copied = target/Path(output['nativePath']).name
                    copied.unlink()
                    # -S suppresses site-packages, while the script directory is the
                    # copied package's actual parent; no checkout path is injected.
                    shutil.copy2(ROOT/'sdk/acceptance/python-consumer.py',disposable/'consumer.py')
                    result = process([RUNTIME['pythonExecutable'],'-S',disposable/'consumer.py'],json.dumps({'requests':[request('capabilities')]}),cwd=disposable,allowed=(1,))
                demand('native' in result.stderr.lower() or 'module' in result.stderr.lower(), 'missing native failed for unrelated reason')
                OBSERVATIONS.append({'control':host+' fresh-process missing native','explicitFailure':result.stderr})
            finally:
                shutil.rmtree(disposable)
                OBSERVATIONS.append({'cleanup':str(disposable),'removed':not disposable.exists()})


def sdk011(control):
    p, expected = next((p,e) for p,e in fixtures() if p.name == 'minimal.feature')
    first_uri, second_uri = 'memory://one.feature', 'memory://two.feature'
    source = p.read_text()
    streams = real_messages()
    run_a = copy.deepcopy(streams['passing']); run_b = copy.deepcopy(streams['passing'])
    def rename_run(envelopes, name):
        for e in envelopes:
            if 'testRunStarted' in e: e['testRunStarted']['id'] = name
            for v in e.values():
                if isinstance(v,dict) and 'testRunStartedId' in v: v['testRunStartedId'] = name
    rename_run(run_a,'owner-A'); rename_run(run_b,'owner-B')
    requests = [compile_request(source,first_uri),compile_request('not gherkin','memory://invalid.feature'),
                compile_request(source,second_uri),rerun_request(run_a),rerun_request(run_b),compile_request(source,first_uri)]
    outputs = hosts(requests)
    for output in outputs.values():
        responses = output['responses']; error(responses[1],'SOURCE_ERROR','memory://invalid.feature')
        for index,uri in ((0,first_uri),(2,second_uri),(5,first_uri)):
            truth = copy.deepcopy(expected)
            truth['gherkinDocument']['uri'] = uri
            for pickle in truth['pickles']: pickle['uri'] = uri
            equal_graph(truth,graph(success(responses[index])))
        compare_plan(success(responses[3]),run_a); compare_plan(success(responses[4]),run_b)
        demand(success(responses[3])['runId'] == 'owner-A' and success(responses[4])['runId'] == 'owner-B','cross-request run leak')
    OBSERVATIONS.append({'sameHostRequests':len(requests),'uriIsolation':[first_uri,second_uri],'runIsolation':['owner-A','owner-B']})
    if control:
        wrong = copy.deepcopy(success(outputs['node']['responses'][3])); wrong['runId'] = 'owner-B'
        reject(lambda:compare_plan(wrong,run_a), 'cross-associated run identity')
        wrong_graph = graph(success(outputs['python']['responses'][0]))
        other_graph = graph(success(outputs['python']['responses'][2]))
        reject(lambda:equal_graph(wrong_graph,other_graph),'cross-associated source URI')


try:
    before = identities()
    dump('oracle-identities-before.json', before)
    demand(not ROOT.is_relative_to(CONSUMER) and not CONSUMER.is_relative_to(ROOT), 'consumer root must be outside checkout')
    globals()['sdk' + CASE.split('-')[1]](MODE == 'control')
    demand(identities() == before, 'original fixture altered by acceptance')
    receipt = {'case':CASE,'mode':MODE,'status':'PASS','observations':OBSERVATIONS,'oracleArtifactsUnchanged':True,'successorSourceIdentities':SUCCESSOR_IDENTITIES}
    dump('receipt.json',receipt)
    print(json.dumps(receipt))
except Exception as exc:
    dump('failure.json',{'case':CASE,'mode':MODE,'status':'FAIL','error':str(exc),'traceback':traceback.format_exc(),'observations':OBSERVATIONS})
    raise
