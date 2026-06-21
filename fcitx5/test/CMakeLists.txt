# Dữ liệu test: đặt .conf của addon/IM theo đúng cấu trúc thư mục fcitx5 trong cây build.
configure_file("${CMAKE_SOURCE_DIR}/src/pinakey.conf.in"
               "${CMAKE_CURRENT_BINARY_DIR}/data/inputmethod/pinakey.conf" @ONLY)
configure_file("${CMAKE_SOURCE_DIR}/src/pinakey-addon.conf.in"
               "${CMAKE_CURRENT_BINARY_DIR}/data/addon/pinakey.conf" @ONLY)

add_executable(testpinakey testpinakey.cpp)
target_link_libraries(testpinakey PRIVATE Fcitx5::Core Fcitx5::Module::TestFrontend)
add_dependencies(testpinakey pinakey)
target_compile_definitions(testpinakey PRIVATE
    TESTING_BINARY_DIR="${CMAKE_BINARY_DIR}"
    PINAKEY_ADDON_SO_DIR="$<TARGET_FILE_DIR:pinakey>"
    PINAKEY_TEST_DATA_DIR="${CMAKE_CURRENT_BINARY_DIR}/data"
    FCITX_SYS_PKGDATADIR="${FCITX_INSTALL_PKGDATADIR}")

add_test(NAME testpinakey COMMAND testpinakey)
