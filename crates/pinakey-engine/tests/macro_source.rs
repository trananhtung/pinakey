//! #129: bảng macro phải nạp theo TÊN engine (file `ibus-<name>.macro.text`), không nạp cứng
//! tên quy ước "PinaKey" — engine tạo theo tên khác đọc nhầm macro của PinaKey.
//! File test riêng (tiến trình riêng) vì đụng XDG_CONFIG_HOME.

use std::fs;

use pinakey_engine::EngineCore;

#[test]
fn macro_nap_theo_ten_engine() {
    let dir = std::env::temp_dir().join(format!("pinakey-macrosrc-{}", std::process::id()));
    fs::create_dir_all(dir.join("pinakey")).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &dir);

    // Bẫy: macro của tên quy ước "PinaKey" — code nạp cứng sẽ dính key "bay".
    fs::write(
        dir.join("pinakey/ibus-PinaKey.macro.text"),
        "bay:sai nguồn\n",
    )
    .unwrap();
    // Macro của tên riêng.
    fs::write(
        dir.join("pinakey/ibus-MacroSrcTest.macro.text"),
        "btw:by the way\n",
    )
    .unwrap();
    fs::write(
        dir.join("pinakey/ibus-MacroSrcTest.config.json"),
        format!(
            "{{\"IBflags\":{}}}",
            pinakey_config::flags::IB_MACRO_ENABLED
        ),
    )
    .unwrap();

    let cfg = pinakey_config::load_config("MacroSrcTest");
    let core = EngineCore::new_named(cfg, "MacroSrcTest");
    assert!(
        core.macro_table.has_key("btw"),
        "engine theo tên phải nạp macro của chính tên đó"
    );
    assert!(
        !core.macro_table.has_key("bay"),
        "engine theo tên KHÔNG được nạp macro của PinaKey"
    );

    let _ = fs::remove_dir_all(&dir);
}
