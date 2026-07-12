//! #98: `reload_config` phải nạp lại đúng NGUỒN cấu hình lúc tạo engine — không nạp cứng
//! tên "PinaKey" (âm thầm thay toàn bộ config của engine tạo theo tên khác / từ JSON tiêm).
//! Chạy trong file riêng (tiến trình riêng) vì test đụng biến môi trường XDG_CONFIG_HOME.

use std::fs;

use pinakey_engine::EngineCore;

fn write_cfg(dir: &std::path::Path, name: &str, input_method: &str) {
    fs::write(
        dir.join(format!("pinakey/ibus-{name}.config.json")),
        format!("{{\"InputMethod\":\"{input_method}\"}}"),
    )
    .unwrap();
}

#[test]
fn reload_theo_dung_nguon_luc_tao() {
    let dir = std::env::temp_dir().join(format!("pinakey-reloadsrc-{}", std::process::id()));
    fs::create_dir_all(dir.join("pinakey")).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &dir);

    // Nhử bẫy: file của tên quy ước "PinaKey" tồn tại với VIQR — code nạp cứng sẽ dính nó.
    write_cfg(&dir, "PinaKey", "VIQR");

    // 1) Engine tạo từ config TIÊM trực tiếp (như pk_engine_new_from_json): reload không có
    //    file nguồn để đọc lại → phải GIỮ NGUYÊN config, không bị thay bằng file "PinaKey".
    let mut injected = pinakey_config::default_cfg();
    injected.input_method = "VNI".to_string();
    let mut core = EngineCore::new(injected);
    core.reload_config();
    assert_eq!(
        core.config().input_method,
        "VNI",
        "engine tiêm JSON bị reload thay config bằng file PinaKey"
    );

    // 2) Engine tạo THEO TÊN khác: reload phải đọc lại đúng file của tên đó (kể cả khi file
    //    đổi nội dung sau khi tạo), không đọc file "PinaKey".
    write_cfg(&dir, "ReloadSrcTest", "VNI");
    let cfg = pinakey_config::load_config("ReloadSrcTest");
    assert_eq!(cfg.input_method, "VNI");
    let mut named = EngineCore::new_named(cfg, "ReloadSrcTest");
    write_cfg(&dir, "ReloadSrcTest", "Telex 2"); // người dùng sửa config
    named.reload_config();
    assert_eq!(
        named.config().input_method,
        "Telex 2",
        "engine theo tên phải nạp lại đúng file của tên đó"
    );

    let _ = fs::remove_dir_all(&dir);
}
