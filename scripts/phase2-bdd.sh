#!/usr/bin/env bash
# Run only after the portable parser phase is implemented.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --test portability -- --tags @phase2
bash scripts/phase1-bdd.sh
