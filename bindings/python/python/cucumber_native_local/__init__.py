"""Native compiler and rerun planning for the local GQAdonis Cucumber SDK.

The extension is required at import time. No parser or subprocess fallback is
provided. Raw requests return protocol envelopes; conveniences raise SdkError.
"""

import json
from typing import Any, Dict, Optional, cast

from . import _native
from ._types import (
    Attempt,
    Baselines,
    Capabilities,
    CompiledSource,
    ErrorCode,
    ErrorPayload,
    RerunEntry,
    RerunPlan,
    SdkResult,
    SourceFormat,
    Status,
)

__all__ = [
    "Attempt",
    "Baselines",
    "Capabilities",
    "CompiledSource",
    "ErrorCode",
    "ErrorPayload",
    "RerunEntry",
    "RerunPlan",
    "SdkError",
    "SdkResult",
    "SourceFormat",
    "Status",
    "capabilities",
    "compile_source",
    "request",
    "rerun",
]


class SdkError(Exception):
    """A Rust SDK error, with code and optional wire context preserved.

    ``line`` is an optional decimal string for a physical NDJSON input line,
    not a location inferred from a parser's error message. ``context`` contains
    precisely the optional fields supplied by Rust, using their wire names.
    """

    code: ErrorCode
    message: str
    context: Dict[str, str]
    uri: Optional[str]
    line: Optional[str]
    reference_kind: Optional[str]
    reference_id: Optional[str]

    def __init__(self, error: ErrorPayload) -> None:
        self.code = error["code"]
        self.message = error["message"]
        self.context = {
            key: value
            for key, value in error.items()
            if key not in ("code", "message")
        }
        self.uri = error.get("uri")
        self.line = error.get("line")
        self.reference_kind = error.get("referenceKind")
        self.reference_id = error.get("referenceId")
        super().__init__(self.message)


def request(raw: str) -> str:
    """Return Rust's JSON response verbatim, including protocol error envelopes.

    Supply JSON text. Non-string Python arguments raise the native binding's
    TypeError; malformed JSON strings are handled by Rust's request contract.
    Every call uses a fresh Rust SDK instance with no shared mutable session.
    """
    return _native.request(raw)


def _invoke(operation: str, payload: Dict[str, Any]) -> SdkResult:
    response = json.loads(
        request(json.dumps({"version": 1, "operation": operation, "input": payload}))
    )
    if not response["ok"]:
        raise SdkError(response["error"])
    return cast(SdkResult, response["result"])


def capabilities() -> Capabilities:
    """Return the native protocol and compatibility targets, not parity claims."""
    return cast(Capabilities, _invoke("capabilities", {}))


def compile_source(
    source: str,
    uri: str,
    format: SourceFormat = "gherkin",
    default_language: str = "en",
) -> CompiledSource:
    """Compile original text in Rust, retaining source graph fields and identity.

    No filesystem reads or syntax guessing occur. The URI is a caller-supplied
    identity and may be empty; it is returned without host normalization.
    """
    return cast(
        CompiledSource,
        _invoke(
            "compile",
            {
                "source": source,
                "uri": uri,
                "format": format,
                "defaultLanguage": default_language,
            },
        ),
    )


def rerun(ndjson: str, separator: str = "\n") -> RerunPlan:
    """Plan reruns in Rust from untouched original Messages NDJSON text.

    Attempts and line numbers remain decimal strings. An empty entries list
    means no rerun selection; runSuccess independently describes the run result.
    """
    return cast(RerunPlan, _invoke("rerun", {"ndjson": ndjson, "separator": separator}))
