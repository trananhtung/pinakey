#!/usr/bin/env bash
# Tạo lại header C-ABI từ nguồn Rust bằng cbindgen.
# Header (crates/pinakey-ffi/include/pinakey_ffi.h) được commit vào repo để bản build C++/CMake
# không phụ thuộc cbindgen lúc biên dịch. Chạy lại script này mỗi khi đổi C-ABI trong lib.rs.
#
#   cargo install cbindgen   # nếu chưa có
#   tools/gen-ffi-header.sh
set -euo pipefail
cd "$(dirname "$0")/.."
cbindgen --config crates/pinakey-ffi/cbindgen.toml \
         --crate pinakey-ffi \
         --output crates/pinakey-ffi/include/pinakey_ffi.h
echo "Đã tạo crates/pinakey-ffi/include/pinakey_ffi.h"
