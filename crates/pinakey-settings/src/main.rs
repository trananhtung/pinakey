//! Binary của giao diện thiết lập PinaKey.
//!
//! - mặc định (build với `--features gui`): mở cửa sổ thiết lập đồ họa.
//! - `--dump`: in cấu hình hiện tại ra stdout rồi thoát (chạy được không cần màn hình).

use pinakey_settings::SettingsController;

/// Tên engine dùng để tìm file cấu hình (`~/.config/pinakey/ibus-PinaKey.config.json`).
const ENGINE_NAME: &str = "PinaKey";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--dump") {
        let ctrl = SettingsController::load(ENGINE_NAME);
        println!("{:#?}", ctrl.config());
        return;
    }

    #[cfg(feature = "gui")]
    {
        if let Err(e) = pinakey_settings::gui::run(ENGINE_NAME) {
            eprintln!("Lỗi mở giao diện thiết lập: {e}");
            std::process::exit(1);
        }
    }
    #[cfg(not(feature = "gui"))]
    {
        eprintln!(
            "Bản build này không kèm GUI. Dùng `pinakey-settings --dump` để xem cấu hình, \
             hoặc build lại với `--features gui`."
        );
    }
}
