'use strict';

// Missing/incompatible native binaries must fail visibly at module load.
const native = require('./cucumber_sdk.node');

class SdkError extends Error {
  constructor(error) {
    super(error.message);
    this.name = 'SdkError';
    this.code = error.code;
    this.context = {};
    for (const key of ['uri', 'line', 'referenceKind', 'referenceId']) {
      if (Object.prototype.hasOwnProperty.call(error, key)) {
        this[key] = error[key];
        this.context[key] = error[key];
      }
    }
  }
}

// Preserve the raw request and response strings, including original NDJSON integers.
const request = native.request;

function invoke(operation, input) {
  const response = JSON.parse(request(JSON.stringify({ version: 1, operation, input })));
  if (!response.ok) {
    throw new SdkError(response.error);
  }
  return response.result;
}

function compile(input) {
  return invoke('compile', input);
}

function rerun(input) {
  return invoke('rerun', input);
}

function capabilities() {
  return invoke('capabilities', {});
}

module.exports = { request, compile, rerun, capabilities, SdkError };
