/*
 * Test fingerprint file cho watcher live-reload (issue #90): phải phát hiện thay đổi
 * kể cả khi nhiều lần ghi rơi vào cùng một giây mtime. GPL-3.0-or-later.
 */
#include "../src/filewatch.h"

#include <fcitx-utils/log.h>

#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <string>

#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace {

void writeFile(const std::string &path, const std::string &content) {
    std::ofstream out(path, std::ios::trunc | std::ios::binary);
    out << content;
}

// Ép mtime về đúng (sec, nsec) cho trước để mô phỏng hai lần ghi cùng giây.
void setMtime(const std::string &path, time_t sec, long nsec) {
    struct timespec times[2];
    times[0].tv_sec = sec;
    times[0].tv_nsec = nsec;
    times[1].tv_sec = sec;
    times[1].tv_nsec = nsec;
    FCITX_ASSERT(utimensat(AT_FDCWD, path.c_str(), times, 0) == 0);
}

} // namespace

int main() {
    using fcitx::pinakey::FileFingerprint;
    using fcitx::pinakey::fileFingerprint;

    char tmpl[] = "/tmp/pinakey-filewatch-XXXXXX";
    const char *dir = mkdtemp(tmpl);
    FCITX_ASSERT(dir != nullptr);
    const std::string base = dir;
    const std::string f1 = base + "/config.json";
    const std::string f2 = base + "/config.json.new";

    // File chưa tồn tại → fingerprint "rỗng", ổn định giữa hai lần gọi.
    FCITX_ASSERT(fileFingerprint(f1) == fileFingerprint(f1));

    // Ghi lần 1, ghim mtime tại (1000000s, 100ns).
    writeFile(f1, "a");
    setMtime(f1, 1000000, 100);
    const FileFingerprint fp1 = fileFingerprint(f1);

    // File xuất hiện phải khác trạng thái "chưa tồn tại".
    FCITX_ASSERT(!(fp1 == fileFingerprint(base + "/khong-ton-tai")));

    // Không đổi gì → fingerprint ổn định.
    FCITX_ASSERT(fp1 == fileFingerprint(f1));

    // Ghi lần 2 CÙNG GIÂY, cùng kích thước, chỉ khác nanosecond → phải khác (lõi issue #90).
    writeFile(f1, "b");
    setMtime(f1, 1000000, 200);
    const FileFingerprint fp2 = fileFingerprint(f1);
    FCITX_ASSERT(!(fp2 == fp1));

    // Ghi lần 3 cùng (sec, nsec) nhưng kích thước khác → phải khác.
    writeFile(f1, "bb");
    setMtime(f1, 1000000, 200);
    const FileFingerprint fp3 = fileFingerprint(f1);
    FCITX_ASSERT(!(fp3 == fp2));

    // Atomic rename (inode mới) với cùng (sec, nsec) và cùng kích thước → phải khác.
    writeFile(f2, "cc");
    setMtime(f2, 1000000, 200);
    FCITX_ASSERT(std::rename(f2.c_str(), f1.c_str()) == 0);
    setMtime(f1, 1000000, 200);
    const FileFingerprint fp4 = fileFingerprint(f1);
    FCITX_ASSERT(!(fp4 == fp3));

    // File bị xoá → quay về trạng thái "rỗng", khác lúc còn tồn tại.
    FCITX_ASSERT(::unlink(f1.c_str()) == 0);
    FCITX_ASSERT(!(fileFingerprint(f1) == fp4));

    ::rmdir(dir);
    return 0;
}
