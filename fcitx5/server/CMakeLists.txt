# Daemon uinput đặc quyền (issue #28). Chỉ C++ thuần + libc, không phụ thuộc fcitx5.
add_executable(pinakey-uinput-server pinakey-server.cpp)
install(TARGETS pinakey-uinput-server DESTINATION "${CMAKE_INSTALL_BINDIR}")

# Đóng gói an toàn:
#  - udev: cấp ACL /dev/uinput cho user ở seat đang hoạt động (uaccess), tải sẵn module uinput.
#  - systemd user service: chạy daemon dưới quyền chính người dùng (khớp SO_PEERCRED).
install(FILES 99-pinakey-uinput.rules
        DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/udev/rules.d")
install(FILES pinakey-uinput.conf
        DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/modules-load.d")
install(FILES pinakey-uinput-server.service
        DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/systemd/user")
