#!/usr/bin/env bash
# Chạy benchmark độ trễ lõi PinaKey (issue #71) — xem docs/BENCHMARK.md để đọc kết quả.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== Máy đo =="
if command -v lscpu >/dev/null 2>&1; then
    lscpu | grep -m1 'Model name' || true
elif command -v sysctl >/dev/null 2>&1; then
    sysctl -n machdep.cpu.brand_string 2>/dev/null || true # macOS
fi
rustc --version
echo

cargo bench -p pinakey-engine "$@"
