#!/usr/bin/env bash
# Run only at the completed phase boundary. No sibling parser checkout required.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --test compatibility -- --tags @phase1
