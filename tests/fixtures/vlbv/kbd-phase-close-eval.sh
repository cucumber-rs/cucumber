#!/usr/bin/env bash
# kbd-phase-close-eval.sh — terminal evaluator at the phase close/continue
# boundary (issue #184 subset; canonical authority per issue #192).
#
# Consumes the phase's adopted scenario RESULTS evidence
# (boundary-results.json) — never task counters, implementationComplete, or
# READY — and records ONE canonical disposition through the typed runtime
# writer `prometheus kbd phase close`:
#
#   PASS    every bound scenario has valid PASS evidence with bound identity
#           => the runtime closes the phase (status complete) atomically;
#              re-evaluation of the same evidence is ALREADY-CLOSED; a closed
#              phase refuses further execute dispatch and successor starts
#              need a signed authorization.
#   FAIL    a product oracle was unmet after valid execution => REFUSED_FAIL.
#   BLOCKED evidence missing/stale/invalid, validator/env unavailable
#           => REFUSED_BLOCKED.
#
# The required scenario set and budget ceilings come from the BOUND contract
# snapshot (.kbd-orchestrator/phases/<id>/adopted/<digest>.json, verified by
# sha256), never from the mutable working contract: editing the working file,
# importing scenarios, or adding "approved" fields cannot change blocking
# scope. Evidence entries for scenarios outside the bound contract are
# reported and ignored. NOT_EXERCISED never counts as PASS. Free-text decision
# summaries are never read as closure.
#
# Usage:
#   kbd-phase-close-eval.sh --phase <id>
#       [--evidence-file <f>]     default <phase-dir>/boundary-results.json
#       [--status-file <f>]       fixture for `prometheus kbd status --json`
#       [--disposition-file <f>]  fixture for `prometheus kbd phase disposition --json`
#       [--decision-sink <f>]     append the disposition that would be recorded
#                                 to a file instead of the canonical writer
#                                 (scratch isolation; disclosed seam)
#
# Exit codes: 0 closed/already-closed; 1 usage/io; 2 BLOCKED refusal;
#             3 FAIL refusal; 5 canonical writer failed (nothing claimed);
#             6 phase already closed under a different evidence identity.
set -euo pipefail

PHASE=""; EVIDENCE=""; STATUS_FILE=""; DISPOSITION_FILE=""; SINK=""
while [ $# -gt 0 ]; do
  case "$1" in
    --phase) PHASE="${2:?}"; shift 2 ;;
    --evidence-file) EVIDENCE="${2:?}"; shift 2 ;;
    --status-file) STATUS_FILE="${2:?}"; shift 2 ;;
    --disposition-file) DISPOSITION_FILE="${2:?}"; shift 2 ;;
    --decision-sink) SINK="${2:?}"; shift 2 ;;
    -h|--help) sed -n '1,40p' "$0"; exit 0 ;;
    *) printf 'kbd-phase-close-eval: unknown argument %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$PHASE" ] || { printf 'usage: kbd-phase-close-eval.sh --phase <id>\n' >&2; exit 1; }
command -v jq >/dev/null || { printf 'jq is required\n' >&2; exit 1; }

ROOT="$PWD"
while [ -n "$ROOT" ] && [ "$ROOT" != "/" ] && [ ! -d "$ROOT/.kbd-orchestrator" ]; do ROOT="$(dirname "$ROOT")"; done
[ -d "$ROOT/.kbd-orchestrator" ] || { printf 'no .kbd-orchestrator above %s\n' "$PWD" >&2; exit 1; }
PHASE_DIR="$ROOT/.kbd-orchestrator/phases/$PHASE"
[ -d "$PHASE_DIR" ] || { printf 'BLOCKED missing=phase-dir(%s)\n' "$PHASE_DIR"; exit 2; }
[ -n "$EVIDENCE" ] || EVIDENCE="$PHASE_DIR/boundary-results.json"
ENFORCED_DIMS="timeMinutes toolCalls repairAttempts noProgress"
ERRF="$(mktemp)"; trap 'rm -f "$ERRF"' EXIT

read_fixture_or() { # <fixture-file> <command...>
  local fixture="$1"; shift
  if [ -n "$fixture" ]; then cat "$fixture"; else "$@" 2>/dev/null; fi
}
STATUS_JSON="$(read_fixture_or "$STATUS_FILE" prometheus kbd --path "$ROOT" status --json)" || {
  printf 'BLOCKED missing=canonical-status(prometheus kbd status failed)\n'; exit 2; }
DISP_JSON="$(read_fixture_or "$DISPOSITION_FILE" prometheus kbd --path "$ROOT" phase disposition --id "$PHASE" --json)" || DISP_JSON=""
printf '%s' "$DISP_JSON" | jq -e '.schema == "kbd.phase-disposition/v1"' >/dev/null 2>&1 || {
  printf 'BLOCKED missing=canonical-disposition(prometheus kbd phase disposition failed for %s)\n' "$PHASE"; exit 2; }
S_RUN="$(printf '%s' "$STATUS_JSON" | jq -r '.runId // empty')"
S_PROJECT="$(printf '%s' "$STATUS_JSON" | jq -r '.projectId // empty')"
TOKEN="$(printf '%s' "$DISP_JSON" | jq -r '.disposition')"
EV_TRUSTED=0

# Record one disposition through the typed writer. Prints nothing; returns the
# writer's exit code (0 recorded or identical no-op).
record() { # <pass|fail|blocked> <reason-code|"">
  local disposition="$1" reason="$2" args
  args=(--id "$PHASE" --disposition "$disposition")
  [ -n "$reason" ] && args+=(--reason-code "$reason")
  [ "$EV_TRUSTED" -eq 1 ] && args+=(--evidence-file "$EVIDENCE")
  if [ -n "$SINK" ]; then
    printf 'PHASE-CLOSE phase=%s disposition=%s reason=%s evidence=%s\n' "$PHASE" "$disposition" \
      "${reason:-none}" "$([ "$EV_TRUSTED" -eq 1 ] && shasum -a 256 "$EVIDENCE" | awk '{print "sha256:" $1}' || echo none)" >> "$SINK"
    return 0
  fi
  prometheus kbd --path "$ROOT" phase close "${args[@]}" >/dev/null 2>"$ERRF"
}

# Derived closure view (VLBV CR-21): closure.json + closure-packet.md beside the
# evidence. Best effort and never changes the verdict or exit code.
closure_packet() { # <disposition> <reason>
  [ -n "${SNAPSHOT:-}" ] && [ -f "${SNAPSHOT:-}" ] || return 0
  command -v node >/dev/null 2>&1 || { printf 'WARN closure-packet not generated for phase=%s (node unavailable)\n' "$PHASE"; return 0; }
  # --project-root lets the packet run the passive pre-authorization artifact
  # scan (VLBV CR-33) over the contract's runnerOutputGlobs; report only.
  if ! node "$(dirname "${BASH_SOURCE[0]}")/kbd-closure-packet.mjs" --snapshot "$SNAPSHOT" --evidence "$EVIDENCE" \
    --out-dir "$PHASE_DIR" --project-root "$ROOT" --disposition "$1" --reason "$2" >/dev/null 2>"$ERRF.packet" \
    || [ ! -f "$PHASE_DIR/closure-packet.md" ]; then
    # Visible, never verdict-changing: the canonical disposition is the authority.
    printf 'WARN closure-packet not generated for phase=%s (%s); rerun: node %s --snapshot %s --evidence %s --out-dir %s --disposition %s\n' \
      "$PHASE" "$(tr '\n' ' ' < "$ERRF.packet" 2>/dev/null | cut -c1-160)" "$(dirname "${BASH_SOURCE[0]}")/kbd-closure-packet.mjs" "$SNAPSHOT" "$EVIDENCE" "$PHASE_DIR" "$1"
  fi
  rm -f "$ERRF.packet"
}

refuse() { # <verdict:FAIL|BLOCKED> <exit> <reason>
  local verdict="$1" code="$2" reason="$3" rc=0
  local lower; lower="$(printf '%s' "$verdict" | tr 'A-Z' 'a-z')"
  record "$lower" "${reason%%(*}" || rc=$?
  if [ "$rc" -ne 0 ]; then
    printf 'WRITER-FAILED phase=%s verdict=%s reason=%s writer-exit=%s (nothing recorded)\n' "$PHASE" "$verdict" "$reason" "$rc"
    exit 5
  fi
  closure_packet "REFUSED_$verdict" "$reason"
  printf '%s phase=%s reason=%s\n' "$verdict" "$PHASE" "$reason"
  exit "$code"
}

# --- canonical binding: an unbound phase has no adopted scope to close against ---
if [ "$TOKEN" = "UNBOUND" ]; then
  printf 'BLOCKED phase=%s reason=contract-unbound(adopt the phase contract with kbd-contract-adopt.sh first)\n' "$PHASE"
  exit 2
fi
B_DIGEST="$(printf '%s' "$DISP_JSON" | jq -r '.binding.contractDigest')"

# --- evidence presence + JSON validity ---
EV_DIGEST=""
if [ -f "$EVIDENCE" ] && jq empty "$EVIDENCE" 2>/dev/null; then
  EV_DIGEST="$(shasum -a 256 "$EVIDENCE" | awk '{print $1}')"
fi

# --- a closed phase is immutable: same evidence is a no-op, anything else is refused ---
if [ "$TOKEN" = "CLOSED" ]; then
  C_EV="$(printf '%s' "$DISP_JSON" | jq -r '.closure.evidenceDigest')"
  C_RUN="$(printf '%s' "$DISP_JSON" | jq -r '.closure.runId')"
  if [ -n "$EV_DIGEST" ] && [ "$C_EV" = "$EV_DIGEST" ] && [ "$C_RUN" = "$S_RUN" ]; then
    printf 'ALREADY-CLOSED phase=%s run=%s contract=%s evidence=sha256:%s (identical closure already recorded; no-op)\n' \
      "$PHASE" "$S_RUN" "$B_DIGEST" "$EV_DIGEST"
    exit 0
  fi
  printf 'CLOSED-DIFFERENT-IDENTITY phase=%s closedEvidence=sha256:%s offered=sha256:%s (a closed phase is immutable)\n' \
    "$PHASE" "$C_EV" "${EV_DIGEST:-none}"
  exit 6
fi

# --- the bound snapshot defines blocking scope ---
SNAPSHOT="$PHASE_DIR/adopted/$B_DIGEST.json"
if [ ! -f "$SNAPSHOT" ] || [ "$(shasum -a 256 "$SNAPSHOT" | awk '{print $1}')" != "$B_DIGEST" ]; then
  refuse BLOCKED 2 "snapshot-invalid($SNAPSHOT does not hash to the bound digest $B_DIGEST)"
fi
DRIFT_NOTE=""
if [ -f "$PHASE_DIR/adopted-contract.json" ]; then
  W_DIGEST="$(jq -S -c 'del(.identity.contractDigest) | del(.identity.planRevision)' "$PHASE_DIR/adopted-contract.json" 2>/dev/null | shasum -a 256 | awk '{print $1}')"
  [ "$W_DIGEST" = "$B_DIGEST" ] || DRIFT_NOTE=" contract-drift-ignored(working=$W_DIGEST bound=$B_DIGEST)"
fi

[ -f "$EVIDENCE" ] || refuse BLOCKED 2 "evidence-missing($EVIDENCE)"
[ -n "$EV_DIGEST" ] || refuse BLOCKED 2 "evidence-invalid-json($EVIDENCE)"

# --- identity binding: phase, project, run, bound contract digest ---
EV_PHASE="$(jq -r '.phase // empty' "$EVIDENCE")"
[ "$EV_PHASE" = "$PHASE" ] || refuse BLOCKED 2 "evidence-phase-mismatch(evidence=$EV_PHASE gate=$PHASE)"
EV_PROJECT="$(jq -r '.projectId // empty' "$EVIDENCE")"
[ -z "$EV_PROJECT" ] || [ "$EV_PROJECT" = "$S_PROJECT" ] || refuse BLOCKED 2 "evidence-project-mismatch(evidence=$EV_PROJECT canonical=$S_PROJECT)"
EV_RUN="$(jq -r '.runId // empty' "$EVIDENCE")"
[ -n "$EV_RUN" ] && [ "$EV_RUN" = "$S_RUN" ] || refuse BLOCKED 2 "evidence-run-mismatch(evidence=$EV_RUN canonical=$S_RUN)"
EV_CDIGEST="$(jq -r '.contractDigest // empty' "$EVIDENCE")"
[ "$EV_CDIGEST" = "$B_DIGEST" ] || refuse BLOCKED 2 "evidence-contract-digest-mismatch(evidence=$EV_CDIGEST bound=$B_DIGEST)"
EV_TRUSTED=1

# --- freshness: recompute digests of the production sources under evidence ---
SRC_CHECK="$(jq -r '.sourceDigests // {} | to_entries[] | .key + " " + .value' "$EVIDENCE" 2>/dev/null)"
[ -n "$SRC_CHECK" ] || refuse BLOCKED 2 "evidence-source-digests-missing"
STALE=""
while IFS=' ' read -r path digest; do
  [ -n "$path" ] || continue
  if [ ! -f "$ROOT/$path" ]; then STALE="${STALE:+$STALE,}missing($path)"; continue; fi
  NOW="$(shasum -a 256 "$ROOT/$path" | awk '{print $1}')"
  [ "$NOW" = "$digest" ] || STALE="${STALE:+$STALE,}stale($path)"
done <<< "$SRC_CHECK"
[ -z "$STALE" ] || refuse BLOCKED 2 "evidence-stale-sources($STALE)"

# --- entry-level oracle over BOUND scenarios only (entry id == S or S-<sub>) ---
REQUIRED="$(jq -r '[.scenarioMatrix[].id] | join(" ")' "$SNAPSHOT")"
ANALYSIS="$(jq -r --arg req "$REQUIRED" '
  ($req | split(" ")) as $required
  | def owned($e): any($required[]; . as $s | $e.id == $s or ($e.id | startswith($s + "-")));
  [ (.scenarios // [])[] | select(owned(.)) ] as $entries
  | [ (.scenarios // [])[] | select(owned(.) | not) | .id ] as $ignored
  | [ $required[] as $s | select([$entries[] | select(.id == $s or (.id | startswith($s + "-")))] | length == 0) | "missing-scenario:" + $s ] as $missing
  | [ $entries[] | select((.attempted // "" | length) == 0 or (.observed // "" | length) == 0 or (.evidence // "" | length) == 0) | "empty-evidence:" + .id ] as $empty
  | [ $entries[] | select(.status == "FAIL") | .id ] as $failed
  | [ $entries[] | select(.status != "PASS" and .status != "FAIL") | "not-pass:" + .id + "(" + (.status // "null") + ")" ] as $notpass
  | [$missing[], $empty[]] as $blocked1
  | (if ($ignored | length) > 0 then " ignored-unadopted(" + ($ignored | join(",")) + ")" else "" end) as $note
  | if ($failed | length) > 0 then "FAIL " + ($failed | join(",")) + $note
     elif ($blocked1 | length) > 0 then "BLOCKED " + ($blocked1 | join(",")) + $note
     elif ($notpass | length) > 0 then "BLOCKED " + ($notpass | join(",")) + $note
     else "PASS" + $note end
' "$EVIDENCE")"
case "$ANALYSIS" in
  FAIL\ *)    refuse FAIL 3 "product-oracle-unmet(${ANALYSIS#FAIL })" ;;
  BLOCKED\ *) refuse BLOCKED 2 "${ANALYSIS#BLOCKED }" ;;
  PASS*)      SCOPE_NOTE="${ANALYSIS#PASS}" ;;
  *)          refuse BLOCKED 2 "evidence-analysis-error" ;;
esac

# --- budget predicate (bound ceilings vs self-reported cumulative consumption) ---
# Ceilings come from the bound snapshot: base budgets plus additive extensions;
# the latest extension's explicit resultingCeilings wins when present. Values
# are self-reported accounting, NOT independent telemetry. ABOVE a ceiling
# refuses; an enforceable dimension (time, tool calls, repairs, no-progress
# cycles against noProgressLimit) reported as
# unknown refuses, because an unmetered budget cannot be shown to hold;
# exactly AT a ceiling closes work already complete within limits and records
# budget-exhausted. The runtime additionally refuses any decrease below the
# recorded high-water consumption.
jq -e '.budgetConsumption' "$EVIDENCE" >/dev/null 2>&1 \
  || refuse BLOCKED 2 "budget-consumption-missing(contract requires budget evaluation: evidence needs budgetConsumption with source and measured/estimate/unknown classes)"
BC_SRC="$(jq -r '.budgetConsumption.source // ""' "$EVIDENCE")"
[ -n "$BC_SRC" ] || refuse BLOCKED 2 "evidence-budget-source-missing"
CEILINGS="$(jq -c '
  .budgets as $b
  | ([.budgets.extensions[]? | select(.resultingCeilings != null) | .resultingCeilings] | last) as $rc
  | if $rc != null then $rc
    else { timeMinutes: ($b.timeMinutes + ([.budgets.extensions[]?.timeMinutes // 0] | add // 0)),
           toolCalls: ($b.toolCalls + ([.budgets.extensions[]?.toolCalls // 0] | add // 0)),
           repairAttempts: ($b.repairAttempts + ([.budgets.extensions[]?.repairAttempts // 0] | add // 0)),
           noProgressLimit: $b.noProgressLimit } end' "$SNAPSHOT")"
BUDGET_CHECK="$(jq -r --argjson c "$CEILINGS" --arg enforced "$ENFORCED_DIMS" '
  (.budgetConsumption.values // {}) as $v
  | { timeMinutes: "timeMinutes", toolCalls: "toolCalls", repairAttempts: "repairAttempts", noProgress: "noProgressLimit" } as $map
  | [ ($enforced | split(" "))[] as $d
      | select(($v[$d] // null) == null or ($v[$d].class // "unknown") == "unknown" or ($v[$d].value // null) == null) | $d ] as $unmeasured
  | [ $map | keys[] | . as $d
      | ($v[$d] // empty) as $val
      | select($val != null and ($val.class // "unknown") != "unknown" and ($val.value // null) != null)
      | ($c[$map[$d]] // 0) as $ceil
      | if $val.value > $ceil then "exceeded:" + $d + "(" + ($val.value | tostring) + ">" + ($ceil | tostring) + "," + $val.class + ")"
        elif $val.value == $ceil then "at-ceiling:" + $d
        else empty end ] as $hits
  | ([$hits[] | select(startswith("exceeded:"))]) as $ex
  | ([$hits[] | select(startswith("at-ceiling:"))]) as $at
  | if ($unmeasured | length) > 0 then "UNMEASURED " + ($unmeasured | join(","))
    elif ($ex | length) > 0 then "EXCEEDED " + ($ex | join(","))
    elif ($at | length) > 0 then "ATCEILING " + ($at | join(","))
    else "OK" end' "$EVIDENCE")"
PASS_REASON=""
case "$BUDGET_CHECK" in
  UNMEASURED\ *) refuse BLOCKED 2 "budget-unmeasured(${BUDGET_CHECK#UNMEASURED }) source=$BC_SRC" ;;
  EXCEEDED\ *) refuse BLOCKED 2 "budget-exceeded(${BUDGET_CHECK#EXCEEDED }) ceilings=$CEILINGS source=$BC_SRC" ;;
  ATCEILING\ *) PASS_REASON="budget-exhausted" ;;
  OK) ;;
esac

# --- close the phase (typed canonical writer; append-only; no tasks, no successor) ---
N_ENTRIES="$(jq '.scenarios | length' "$EVIDENCE")"
RC=0
record pass "$PASS_REASON" || RC=$?
case "$RC" in
  0) ;;
  6) printf 'CLOSED-DIFFERENT-IDENTITY phase=%s (closed concurrently under another evidence identity)\n' "$PHASE"; exit 6 ;;
  7) printf 'BLOCKED phase=%s reason=runtime-refused-pass(%s)\n' "$PHASE" \
       "$(tr '\n' ' ' < "$ERRF" 2>/dev/null | sed 's/  */ /g')"
     exit 2 ;;
  *) printf 'WRITER-FAILED phase=%s verdict=PASS writer-exit=%s (phase NOT closed)\n' "$PHASE" "$RC"; exit 5 ;;
esac
closure_packet CLOSED "${PASS_REASON:-pass}"
printf 'CLOSED phase=%s verdict=PASS evidence=sha256:%s contract=%s run=%s entries=%s%s%s%s (dispatch for this phase now refused)\n' \
  "$PHASE" "$EV_DIGEST" "$B_DIGEST" "$S_RUN" "$N_ENTRIES" \
  "${PASS_REASON:+ budget-note=$PASS_REASON}" "$SCOPE_NOTE" "$DRIFT_NOTE"
exit 0
