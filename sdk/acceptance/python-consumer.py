"""Copied into a fresh consumer directory; imports only the installed wheel."""
import json
import sys
import cucumber_native_local as sdk
from cucumber_native_local import _native
payload = json.load(sys.stdin)
def respond(raw):
    if not payload.get('facade'):
        return json.loads(sdk.request(raw))
    parsed = json.loads(raw)
    try:
        args = parsed['input']
        if parsed['operation'] == 'compile':
            result = sdk.compile_source(args['source'], args['uri'], format=args['format'], default_language=args['defaultLanguage'])
        elif parsed['operation'] == 'rerun':
            result = sdk.rerun(args['ndjson'], separator=args['separator'])
        else:
            result = sdk.capabilities()
        return {'version':1,'ok':True,'result':result}
    except sdk.SdkError as error:
        return {'version':1,'ok':False,'error':{'code':error.code,'message':error.message,**error.context}}
print(json.dumps({'responses': [respond(raw) for raw in payload['requests']],
                  'modulePath': sdk.__file__, 'nativePath': _native.__file__}))
