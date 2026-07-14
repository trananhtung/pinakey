//! #153: Hyper/Mod3 vật lý (fcitx5 `KeyState::Hyper = Mod3 = 1 << 5`) phải pass-through như mọi
//! tổ hợp điều khiển — không nuốt shortcut, không đụng buffer/preedit.

use pinakey_config::default_cfg;
use pinakey_engine::EngineCore;

#[test]
fn fcitx_physical_hyper_must_pass_through() {
    let mut engine = EngineCore::new(default_cfg());
    for c in "vie".chars() {
        engine.process_key_event(c as u32, 0, 0);
    }

    // fcitx5 KeyState::Hyper / Mod3 = bit 5.
    let (handled, actions) = engine.process_key_event('a' as u32, 0, 1 << 5);
    assert!(!handled, "Hyper+a phải đi thẳng tới ứng dụng");
    assert!(
        actions.is_empty(),
        "Hyper+a không được sửa preedit: {actions:?}"
    );
}

#[test]
fn buffer_intact_after_hyper_shortcut() {
    // Sau khi Hyper+a đi qua mà không đụng buffer, gõ tiếp phải ra đúng từ trước shortcut.
    let mut engine = EngineCore::new(default_cfg());
    for c in "vie".chars() {
        engine.process_key_event(c as u32, 0, 0);
    }
    engine.process_key_event('a' as u32, 0, 1 << 5); // Hyper+a — không được vào buffer
    let (handled, actions) = engine.process_key_event('t' as u32, 0, 0);
    assert!(handled, "'t' nối tiếp buffer 'vie' phải được xử lý");
    let preedit = actions.iter().rev().find_map(|a| match a {
        pinakey_engine::Action::UpdatePreedit { text, .. } => Some(text.clone()),
        _ => None,
    });
    assert_eq!(
        preedit.as_deref(),
        Some("viet"),
        "buffer phải là 'vie' + 't' = 'viet', không lẫn 'a' của shortcut: {actions:?}"
    );
}
