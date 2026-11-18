// Copied next to the installed npm package; no checkout resolution injection.
const fs = require('node:fs');
const sdk = require('cucumber-native-local');
const input = JSON.parse(fs.readFileSync(0, 'utf8'));
const responses = input.requests.map(raw => {
  if (!input.facade) return JSON.parse(sdk.request(raw));
  const parsed = JSON.parse(raw);
  try {
    const result = parsed.operation === 'compile' ? sdk.compile(parsed.input)
      : parsed.operation === 'rerun' ? sdk.rerun(parsed.input) : sdk.capabilities();
    return {version:1,ok:true,result};
  } catch (error) {
    if (!(error instanceof sdk.SdkError)) throw error;
    return {version:1,ok:false,error:{code:error.code,message:error.message,...error.context}};
  }
});
const nativePath = Object.keys(require.cache).find(path => path.endsWith('.node'));
if (!nativePath) throw new Error('Installed native addon was not loaded');
process.stdout.write(JSON.stringify({responses, modulePath:require.resolve('cucumber-native-local'), nativePath}));
