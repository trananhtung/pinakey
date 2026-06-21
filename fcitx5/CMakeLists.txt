# Addon fcitx5 cho PinaKey: vỏ C++ mỏng + lõi engine Rust (staticlib) liên kết tĩnh.
#
# Build độc lập:
#   cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr
#   cmake --build fcitx5/build
#   sudo cmake --install fcitx5/build
cmake_minimum_required(VERSION 3.16)
project(fcitx5-pinakey VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(REQUIRED_FCITX_VERSION 5.0.0)

find_package(ECM REQUIRED)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH} ${CMAKE_MODULE_PATH})
include(GNUInstallDirs)

find_package(Fcitx5Core ${REQUIRED_FCITX_VERSION} REQUIRED)
find_package(Fcitx5Utils ${REQUIRED_FCITX_VERSION} REQUIRED)

include("${FCITX_INSTALL_CMAKECONFIG_DIR}/Fcitx5Utils/Fcitx5CompilerSettings.cmake")

# ---------------------------------------------------------------------------
# Lõi Rust: build bằng cargo, nhập vào như một thư viện tĩnh IMPORTED.
# (Không dùng Corrosion để build tự chứa, không cần mạng lúc configure.)
# ---------------------------------------------------------------------------
find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(PINAKEY_REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/..")
set(PINAKEY_RUST_LIB "${PINAKEY_REPO_ROOT}/target/release/libpinakey_ffi.a")
set(PINAKEY_RUST_INCLUDE "${PINAKEY_REPO_ROOT}/crates/pinakey-ffi/include")

# cargo tự incremental nên target ALL chạy lại mỗi lần build là rẻ.
add_custom_target(pinakey-ffi ALL
    COMMAND ${CARGO_EXECUTABLE} build -p pinakey-ffi --release
    WORKING_DIRECTORY "${PINAKEY_REPO_ROOT}"
    BYPRODUCTS "${PINAKEY_RUST_LIB}"
    COMMENT "Đang build lõi PinaKey (Rust staticlib) bằng cargo"
    VERBATIM)

add_library(PinaKey::Ffi STATIC IMPORTED GLOBAL)
add_dependencies(PinaKey::Ffi pinakey-ffi)
set_target_properties(PinaKey::Ffi PROPERTIES
    IMPORTED_LOCATION "${PINAKEY_RUST_LIB}"
    INTERFACE_INCLUDE_DIRECTORIES "${PINAKEY_RUST_INCLUDE}"
    # Native deps của Rust std khi liên kết tĩnh (cargo --print native-static-libs).
    INTERFACE_LINK_LIBRARIES "gcc_s;util;rt;pthread;m;dl;c")

add_subdirectory(src)

# Test tích hợp (chỉ build nếu module testfrontend của fcitx5 có sẵn — gói -dev/-modules).
option(PINAKEY_BUILD_TESTS "Build fcitx5 integration tests" ON)
if(PINAKEY_BUILD_TESTS)
    find_package(Fcitx5ModuleTestFrontend QUIET)
    if(TARGET Fcitx5::Module::TestFrontend)
        enable_testing()
        add_subdirectory(test)
    else()
        message(STATUS "Không thấy Fcitx5::Module::TestFrontend — bỏ qua test tích hợp.")
    endif()
endif()
