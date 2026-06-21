add_library(pinakey MODULE pinakey.cpp)
target_link_libraries(pinakey PRIVATE Fcitx5::Core PinaKey::Ffi)
add_dependencies(pinakey pinakey-ffi)
# Module fcitx5 phải tên `pinakey.so` (không tiền tố lib), khớp Library=pinakey trong addon conf.
set_target_properties(pinakey PROPERTIES PREFIX "")

install(TARGETS pinakey DESTINATION "${FCITX_INSTALL_LIBDIR}/fcitx5")

# File đăng ký input method.
configure_file(pinakey.conf.in "${CMAKE_CURRENT_BINARY_DIR}/pinakey.conf" @ONLY)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/pinakey.conf"
        DESTINATION "${FCITX_INSTALL_DATADIR}/fcitx5/inputmethod")

# File metadata của addon (đổi tên thành pinakey.conf trong thư mục addon).
configure_file(pinakey-addon.conf.in "${CMAKE_CURRENT_BINARY_DIR}/pinakey-addon.conf" @ONLY)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/pinakey-addon.conf"
        RENAME pinakey.conf
        DESTINATION "${FCITX_INSTALL_PKGDATADIR}/addon")
