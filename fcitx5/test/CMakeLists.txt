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

# Test 4: fingerprint file cho watcher live-reload (#90, không cần engine).
add_executable(testfilewatch testfilewatch.cpp)
target_link_libraries(testfilewatch PRIVATE Fcitx5::Core)
add_test(NAME testfilewatch COMMAND testfilewatch)

# Test 5: client uinput — reconnect có throttle khi daemon khởi động muộn/restart (#91).
add_executable(testuinputclient testuinputclient.cpp)
target_link_libraries(testuinputclient PRIVATE Fcitx5::Core)
add_test(NAME testuinputclient COMMAND testuinputclient)

# Test 6: resolve đường socket uinput filesystem trong XDG_RUNTIME_DIR (#72).
add_executable(testsocketpath testsocketpath.cpp)
target_link_libraries(testsocketpath PRIVATE Fcitx5::Core)
add_test(NAME testsocketpath COMMAND testsocketpath)

# Test 7: xác thực client daemon uinput — SO_PEERCRED + allowlist exe (#72).
add_executable(testpeerauth testpeerauth.cpp)
target_link_libraries(testpeerauth PRIVATE Fcitx5::Core)
add_test(NAME testpeerauth COMMAND testpeerauth)

# Test 8: tích hợp uinput+ACK qua fcitx5 thật + daemon giả lập — vòng ACK, timeout replay (#96).
add_executable(testuinputack testuinputack.cpp)
target_link_libraries(testuinputack PRIVATE Fcitx5::Core)
add_dependencies(testuinputack pinakey)
target_compile_definitions(testuinputack PRIVATE ${PINAKEY_TEST_DEFS})
add_test(NAME testuinputack COMMAND testuinputack)

# Test 9: bind socket server — thư mục 0700, socket 0600, dọn socket cũ (#72).
add_executable(testserversocket testserversocket.cpp)
target_link_libraries(testserversocket PRIVATE Fcitx5::Core)
add_test(NAME testserversocket COMMAND testserversocket)
