#!/usr/bin/env bash
# Chạy benchmark độ trễ lõi PinaKey (issue #71) — xem docs/BENCHMARK.md để đọc kết quả.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== Máy đo =="
lscpu | grep -m1 'Model name' || true
rustc --version
echo

cargo bench -p pinakey-engine "$@"
