# Dữ liệu test: đặt .conf của addon/IM theo đúng cấu trúc thư mục fcitx5 trong cây build.
configure_file("${CMAKE_SOURCE_DIR}/src/pinakey.conf.in"
               "${CMAKE_CURRENT_BINARY_DIR}/data/inputmethod/pinakey.conf" @ONLY)
configure_file("${CMAKE_SOURCE_DIR}/src/pinakey-addon.conf.in"
               "${CMAKE_CURRENT_BINARY_DIR}/data/addon/pinakey.conf" @ONLY)

set(PINAKEY_TEST_DEFS
    TESTING_BINARY_DIR="${CMAKE_BINARY_DIR}"
    PINAKEY_ADDON_SO_DIR="$<TARGET_FILE_DIR:pinakey>"
    PINAKEY_TEST_DATA_DIR="${CMAKE_CURRENT_BINARY_DIR}/data"
    FCITX_SYS_PKGDATADIR="${FCITX_INSTALL_PKGDATADIR}")

# Test 1: preedit + commit qua testfrontend của fcitx5.
add_executable(testpinakey testpinakey.cpp)
target_link_libraries(testpinakey PRIVATE Fcitx5::Core Fcitx5::Module::TestFrontend)
add_dependencies(testpinakey pinakey)
target_compile_definitions(testpinakey PRIVATE ${PINAKEY_TEST_DEFS})
add_test(NAME testpinakey COMMAND testpinakey)

# Test 2: gõ không gạch chân (SurroundingText) qua InputContext giả lập ô văn bản.
add_executable(testpinakey_nounderline testpinakey_nounderline.cpp)
target_link_libraries(testpinakey_nounderline PRIVATE Fcitx5::Core)
add_dependencies(testpinakey_nounderline pinakey)
target_compile_definitions(testpinakey_nounderline PRIVATE ${PINAKEY_TEST_DEFS})
add_test(NAME testpinakey_nounderline COMMAND testpinakey_nounderline)

# Test 3: hàm tiện ích UTF-8 dùng chung của addon (không cần engine).
add_executable(testutf8util testutf8util.cpp)
target_link_libraries(testutf8util PRIVATE Fcitx5::Core)
add_test(NAME testutf8util COMMAND testutf8util)
