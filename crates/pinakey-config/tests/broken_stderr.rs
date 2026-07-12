//! #99: đường cảnh báo của load_config KHÔNG được panic khi stderr hỏng — `eprintln!` panic
//! khi ghi thất bại, mà profile release đặt `panic = "abort"` và staticlib nhúng vào fcitx5:
//! một lời "cảnh báo" sẽ giết cả bộ gõ. Test hướng stderr vào /dev/full (mọi write trả
//! ENOSPC) rồi nạp config hỏng: phải trả về mặc định êm ả.

use std::fs;
use std::os::unix::io::AsRawFd;
use std::process::Command;

/// Vỏ ngoài: cargo test mặc định CAPTURE eprintln! (không chạm fd 2 thật) nên phải chạy phần
/// thân trong tiến trình con với --nocapture; con panic/abort → exit khác 0 → test fail.
#[test]
fn load_config_khong_panic_khi_stderr_hong() {
    if std::env::var("PK_BROKEN_STDERR_CHILD").is_ok() {
        child_body();
        return;
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .env("PK_BROKEN_STDERR_CHILD", "1")
        .args([
            "--nocapture",
            "--exact",
            "load_config_khong_panic_khi_stderr_hong",
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "tiến trình con panic khi stderr hỏng: {status}"
    );
}

fn child_body() {
    // Config hỏng trong XDG_CONFIG_HOME cách ly (test chạy một mình trong file này nên
    // set_var an toàn).
    let dir = std::env::temp_dir().join(format!("pinakey-brokenstderr-{}", std::process::id()));
    fs::create_dir_all(dir.join("pinakey")).unwrap();
    fs::write(
        dir.join("pinakey/ibus-BrokenStderrTest.config.json"),
        b"{ json hong",
    )
    .unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &dir);

    // Hướng fd 2 vào /dev/full: mọi write vào stderr thất bại (ENOSPC) một cách tất định.
    let devfull = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let saved = unsafe { libc::dup(2) };
    assert!(saved >= 0);
    assert!(unsafe { libc::dup2(devfull.as_raw_fd(), 2) } >= 0);

    // Đường cảnh báo "config hỏng → backup + dùng mặc định" phải sống sót.
    let cfg = pinakey_config::load_config("BrokenStderrTest");

    // Khôi phục stderr để harness còn in kết quả.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }

    assert_eq!(
        cfg.input_method,
        pinakey_config::load_config("KhongTonTai--").input_method
    );
    let _ = fs::remove_dir_all(&dir);
}
