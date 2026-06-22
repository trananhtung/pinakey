# Addon fcitx5 cho PinaKey: vỏ C++ mỏng + lõi engine Rust (staticlib) liên kết tĩnh.
#
# Build độc lập:
#   cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr
#   cmake --build fcitx5/build
#   sudo cmake --install fcitx5/build
cmake_minimum_required(VERSION 3.16)

# Version: nguồn duy nhất là Cargo.toml của workspace (tránh lệch giữa .deb/CMake và crate Rust).
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/../Cargo.toml" _pinakey_cargo_toml)
string(REGEX MATCH "version[ \t]*=[ \t]*\"([0-9]+\\.[0-9]+\\.[0-9]+)\"" _pinakey_ver_match "${_pinakey_cargo_toml}")
if(NOT CMAKE_MATCH_1)
    message(FATAL_ERROR "Không đọc được version từ ../Cargo.toml")
endif()
project(fcitx5-pinakey VERSION ${CMAKE_MATCH_1} LANGUAGES CXX)

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

# Icon của input method (đồng nhất với logo trang pinakey-web). fcitx5 tra theo Icon=pinakey.
install(FILES icons/pinakey.svg
        DESTINATION "${FCITX_INSTALL_DATADIR}/icons/hicolor/scalable/apps")

# Daemon uinput bơm Backspace cho gõ không gạch chân ở app không hỗ trợ SurroundingText (#28).
# MẶC ĐỊNH TẮT: đây là chế độ THỬ NGHIỆM (chỉ chạy khi đặt env `PINAKEY_UINPUT=1`) và không ổn
# định trên GNOME Wayland (frontend D-Bus không bảo đảm thứ tự xoá/commit → rối ký tự). Mặc định
# terminal dùng preedit; trình duyệt/editor dùng SurroundingText. Bật ON nếu muốn tự thử uinput —
# khi ON sẽ cài kèm binary + udev rule (nạp module kernel `uinput` + cấp ACL /dev/uinput mỗi boot).
option(PINAKEY_BUILD_UINPUT_SERVER "Build the experimental uinput backspace server (opt-in)" OFF)
if(PINAKEY_BUILD_UINPUT_SERVER)
    add_subdirectory(server)
endif()

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

# ---------------------------------------------------------------------------
# Đóng gói deb/rpm (issue #21): `cpack -G DEB` / `cpack -G RPM` trong fcitx5/build.
# ---------------------------------------------------------------------------
set(CPACK_PACKAGE_NAME "fcitx5-pinakey")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_CONTACT "Tran Anh Tung <trananhtung@users.noreply.github.com>")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "PinaKey — bộ gõ tiếng Việt cho fcitx5 (lõi Rust)")
set(CPACK_PACKAGE_HOMEPAGE_URL "https://github.com/trananhtung/pinakey")

# --- DEB ---
set(CPACK_DEBIAN_FILE_NAME DEB-DEFAULT)              # tên gồm version+kiến trúc: fcitx5-pinakey_<ver>_amd64.deb
set(CPACK_DEBIAN_PACKAGE_DEPENDS "fcitx5 (>= 5.0.0)")
# Recommends: công cụ cấu hình (gói riêng trên Ubuntu/Debian) + im-config để bật fcitx5 ở mức phiên.
set(CPACK_DEBIAN_PACKAGE_RECOMMENDS "fcitx5-config-qt | fcitx5-configtool, im-config")
set(CPACK_DEBIAN_PACKAGE_SHLIBDEPS ON)              # tự dò phụ thuộc thư viện động (libfcitx5*, libstdc++…)
set(CPACK_DEBIAN_PACKAGE_SECTION "utils")
set(CPACK_DEBIAN_PACKAGE_CONTROL_EXTRA
    "${CMAKE_CURRENT_SOURCE_DIR}/../packaging/deb/postinst;${CMAKE_CURRENT_SOURCE_DIR}/../packaging/deb/postrm")

# --- RPM ---
set(CPACK_RPM_FILE_NAME RPM-DEFAULT)
set(CPACK_RPM_PACKAGE_REQUIRES "fcitx5 >= 5.0.0")
set(CPACK_RPM_PACKAGE_LICENSE "GPL-3.0-or-later")
include(CPack)

# ---------------------------------------------------------------------------
# Mục tiêu gỡ cài: `sudo cmake --build fcitx5/build --target uninstall`
# (đọc install_manifest.txt do `cmake --install` sinh).
# ---------------------------------------------------------------------------
configure_file("${CMAKE_CURRENT_SOURCE_DIR}/cmake_uninstall.cmake.in"
               "${CMAKE_CURRENT_BINARY_DIR}/cmake_uninstall.cmake" @ONLY)
add_custom_target(uninstall
    COMMAND "${CMAKE_COMMAND}" -P "${CMAKE_CURRENT_BINARY_DIR}/cmake_uninstall.cmake")
