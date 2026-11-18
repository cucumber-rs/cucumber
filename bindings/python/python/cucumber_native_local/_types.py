"""Protocol v1 result types; AST/Pickle objects retain upstream optional fields."""

from typing import Any, Dict, List, Literal, Optional, TypedDict, Union

SourceFormat = Literal["gherkin", "markdown"]
ErrorCode = Literal[
    "INVALID_REQUEST",
    "UNSUPPORTED_VERSION",
    "UNKNOWN_OPERATION",
    "SOURCE_ERROR",
    "INVALID_MESSAGES",
    "INCOMPLETE_RUN",
    "RUN_ALREADY_FINISHED",
    "MISSING_REFERENCE",
    "INTERNAL_ERROR",
]
Status = Literal[
    "UNKNOWN", "PASSED", "SKIPPED", "PENDING", "UNDEFINED", "AMBIGUOUS", "FAILED"
]


class CompiledSource(TypedDict):
    kind: Literal["compiled"]
    uri: str
    format: SourceFormat
    gherkinDocument: Dict[str, Any]
    pickles: List[Dict[str, Any]]


class Attempt(TypedDict):
    startedId: str
    testCaseId: str
    pickleId: str
    attempt: str
    uri: str
    line: str
    status: Status
    completed: bool
    willBeRetried: bool


class RerunEntry(TypedDict):
    uri: str
    lines: List[str]


class RerunPlan(TypedDict):
    kind: Literal["rerun"]
    runId: Optional[str]
    runSuccess: bool
    attempts: List[Attempt]
    entries: List[RerunEntry]
    formatted: str


class Baselines(TypedDict):
    gherkin: str
    cucumberJs: str
    messages: List[str]


class Capabilities(TypedDict):
    kind: Literal["capabilities"]
    protocolVersions: List[int]
    operations: List[str]
    formats: List[SourceFormat]
    baselines: Baselines
    foreignStepExecution: Literal[False]


class _ErrorRequired(TypedDict):
    code: ErrorCode
    message: str


class ErrorPayload(_ErrorRequired, total=False):
    uri: str
    line: str
    referenceKind: str
    referenceId: str


SdkResult = Union[CompiledSource, RerunPlan, Capabilities]
