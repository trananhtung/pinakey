//! Điểm vào của pinakey — chuyển từ `main.go`.
//!
//! IBus khởi chạy engine với `--ibus` (chế độ embedded); file XML component đã cài đặt trỏ tới
//! binary này. `--version` in ra phiên bản.

use pinakey_ibus::dbus::run_embedded;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |name: &str| args.iter().any(|a| a == name);

    if has("--version") || has("-version") {
        println!("{VERSION}");
        return;
    }

    // Cả cách gọi embedded (`--ibus`) lẫn cách gọi mặc định đều chạy engine trên bus IBus.
    // (Chế độ standalone của bản Go còn đăng ký thêm một component descriptor; trong môi trường
    // production, IBus luôn khởi chạy engine qua file XML component đã cài đặt với `--ibus`.)
    if let Err(e) = run_embedded().await {
        eprintln!("pinakey failed: {e}");
        std::process::exit(1);
    }
}
