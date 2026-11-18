/** JSON graph payloads preserve optional upstream fields without inventing defaults. */
export type JsonValue = null | boolean | number | string | JsonValue[] | JsonObject;
export interface JsonObject { [key: string]: JsonValue }
export type SourceFormat = 'gherkin' | 'markdown';

/** Decimal integer strings prevent JavaScript rounding of Rust u64 counters. */
export type DecimalString = string;

export interface CompileInput {
  source: string;
  uri: string;
  format: SourceFormat;
  defaultLanguage: string;
}

export interface Compiled {
  kind: 'compiled';
  uri: string;
  format: SourceFormat;
  gherkinDocument: JsonObject;
  pickles: JsonObject[];
}

export interface RerunInput {
  /** Original NDJSON text: do not parse and reserialize it in JavaScript. */
  ndjson: string;
  separator: string;
}

export type StepStatus =
  | 'UNKNOWN' | 'PASSED' | 'SKIPPED' | 'PENDING'
  | 'UNDEFINED' | 'AMBIGUOUS' | 'FAILED';

export interface Attempt {
  startedId: string;
  testCaseId: string;
  pickleId: string;
  attempt: DecimalString;
  uri: string;
  line: DecimalString;
  status: StepStatus;
  completed: boolean;
  willBeRetried: boolean;
}

export interface RerunEntry {
  uri: string;
  lines: DecimalString[];
}

export interface RerunPlan {
  kind: 'rerun';
  runId: string | null;
  runSuccess: boolean;
  attempts: Attempt[];
  /** Empty entries mean no selected scenarios, never all-source execution. */
  entries: RerunEntry[];
  formatted: string;
}

export interface Capabilities {
  kind: 'capabilities';
  protocolVersions: [1];
  operations: ['capabilities', 'compile', 'rerun'];
  formats: ['gherkin', 'markdown'];
  baselines: {
    gherkin: '43.0.0';
    cucumberJs: '13.3.0';
    messages: ['34.2.1', '35.0.0'];
  };
  foreignStepExecution: false;
}

export type ErrorCode =
  | 'INVALID_REQUEST' | 'UNSUPPORTED_VERSION' | 'UNKNOWN_OPERATION'
  | 'SOURCE_ERROR' | 'INVALID_MESSAGES' | 'INCOMPLETE_RUN'
  | 'RUN_ALREADY_FINISHED' | 'MISSING_REFERENCE' | 'INTERNAL_ERROR';

export interface ErrorContext {
  uri?: string;
  /** When present, may identify an NDJSON physical line rather than a source line. */
  line?: DecimalString;
  referenceKind?: string;
  referenceId?: string;
}

export interface ErrorPayload extends ErrorContext {
  code: ErrorCode;
  message: string;
}

export class SdkError extends Error implements ErrorContext {
  constructor(error: ErrorPayload);
  readonly name: 'SdkError';
  readonly code: ErrorCode;
  readonly context: ErrorContext;
  readonly uri?: string;
  readonly line?: DecimalString;
  readonly referenceKind?: string;
  readonly referenceId?: string;
}

export type SdkRequest =
  | { version: 1; operation: 'capabilities'; input: Record<string, never> }
  | { version: 1; operation: 'compile'; input: CompileInput }
  | { version: 1; operation: 'rerun'; input: RerunInput };

export type SdkResponse =
  | { version: 1; ok: true; result: Compiled | RerunPlan | Capabilities }
  | { version: 1; ok: false; error: ErrorPayload };

/** Native JSON transport. Protocol failures are returned in the response string. */
export function request(raw: string): string;
/** Return the complete Rust graph, or throw SdkError for a protocol failure. */
export function compile(input: CompileInput): Compiled;
/** Compute a lossless plan in Rust, or throw SdkError for a protocol failure. */
export function rerun(input: RerunInput): RerunPlan;
/** Report supported operations and target baselines, not a full-parity claim. */
export function capabilities(): Capabilities;
