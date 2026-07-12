/*
 * Fingerprint file cho watcher live-reload của addon PinaKey. GPL-3.0-or-later.
 */
#ifndef _PINAKEY_FCITX5_FILEWATCH_H_
#define _PINAKEY_FCITX5_FILEWATCH_H_

#include <cstdint>
#include <string>

#include <sys/stat.h>

namespace fcitx::pinakey {

/// Dấu vân tay một file cho watcher (#90): mtime giây KHÔNG đủ phân giải khi file được ghi
/// nhiều lần trong cùng một giây, nên gộp thêm nanosecond, inode (bắt atomic rename) và
/// kích thước. File không tồn tại → mọi trường bằng 0.
struct FileFingerprint {
    uint64_t sec = 0;
    uint64_t nsec = 0;
    uint64_t inode = 0;
    uint64_t size = 0;

    bool operator==(const FileFingerprint &other) const {
        return sec == other.sec && nsec == other.nsec && inode == other.inode &&
               size == other.size;
    }
    bool operator!=(const FileFingerprint &other) const { return !(*this == other); }
};

inline FileFingerprint fileFingerprint(const std::string &path) {
    struct stat st {};
    if (stat(path.c_str(), &st) != 0) {
        return {};
    }
    return {static_cast<uint64_t>(st.st_mtim.tv_sec), static_cast<uint64_t>(st.st_mtim.tv_nsec),
            static_cast<uint64_t>(st.st_ino), static_cast<uint64_t>(st.st_size)};
}

} // namespace fcitx::pinakey

#endif // _PINAKEY_FCITX5_FILEWATCH_H_
