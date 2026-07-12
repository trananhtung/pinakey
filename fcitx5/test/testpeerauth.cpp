/*
 * Test xác thực client của daemon uinput (#72): SO_PEERCRED (UID trùng) + đường dẫn thật
 * của tiến trình (readlink /proc/<pid>/exe) phải là fcitx5 ở prefix chuẩn. GPL-3.0-or-later.
 */
#include "../server/peerauth.h"

#include <fcitx-utils/log.h>

#include <string>

#include <sys/socket.h>
#include <unistd.h>

int main() {
    using pinakey::exeAllowed;
    using pinakey::peerAuthorized;
    using pinakey::peerCredentials;

    // Allowlist binary fcitx5 — so sánh CHÍNH XÁC, không prefix-match lỏng lẻo.
    FCITX_ASSERT(exeAllowed("/usr/bin/fcitx5"));
    FCITX_ASSERT(exeAllowed("/usr/local/bin/fcitx5"));
    FCITX_ASSERT(!exeAllowed("/tmp/fcitx5"));
    FCITX_ASSERT(!exeAllowed("/usr/bin/fcitx5x"));
    FCITX_ASSERT(!exeAllowed("/usr/bin/fcitx5 (deleted)"));
    FCITX_ASSERT(!exeAllowed(""));

    // Phần thuần (đã có UID + exe): cả hai lớp phải cùng đạt.
    FCITX_ASSERT(peerAuthorized(1000, "/usr/bin/fcitx5", 1000));
    FCITX_ASSERT(!peerAuthorized(1001, "/usr/bin/fcitx5", 1000)); // sai UID
    FCITX_ASSERT(!peerAuthorized(1000, "/usr/bin/evil", 1000));   // sai binary

    // SO_PEERCRED trên socketpair: phải đọc ra đúng UID/PID của chính tiến trình test.
    int sv[2];
    FCITX_ASSERT(socketpair(AF_UNIX, SOCK_SEQPACKET, 0, sv) == 0);
    struct ucred cred {};
    FCITX_ASSERT(peerCredentials(sv[0], cred));
    FCITX_ASSERT(cred.uid == getuid());
    FCITX_ASSERT(cred.pid == getpid());

    // Đường đầy đủ qua fd: UID trùng nhưng exe là binary test (không phải fcitx5) → phải
    // TỪ CHỐI — chứng minh lớp exe hoạt động độc lập với lớp UID.
    FCITX_ASSERT(!peerAuthorized(sv[0], getuid()));
    // Sai UID kỳ vọng → từ chối.
    FCITX_ASSERT(!peerAuthorized(sv[0], getuid() + 1));

    ::close(sv[0]);
    ::close(sv[1]);
    return 0;
}
