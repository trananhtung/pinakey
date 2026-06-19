//! Smoke test end-to-end cho **phím tắt bật/tắt tiếng Việt** và **menu thuộc tính** trên bus IBus.
//!
//! Kiểm tra:
//!  - `FocusIn` phát `RegisterProperties` (đăng ký menu lên panel);
//!  - phím tắt mặc định (Ctrl + keyval 126) tắt tiếng Việt -> phím gõ được cho đi qua
//!    (`ProcessKeyEvent` trả `false`); bật lại thì biến đổi hoạt động ("vieetj" -> "việt");
//!  - `PropertyActivate("im_VNI")` không làm engine sập và đăng ký lại menu.
//!
//!     cargo run -p pinakey-ibus --example shortcut_props_smoketest

use std::time::Duration;

use futures_util::StreamExt;
use zbus::zvariant::{OwnedObjectPath, Value};

use pinakey_ibus::dbus::Factory;

const TEST_NAME: &str = "org.freedesktop.IBus.pinakey.shortcutsmoketest";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";
const IBUS_CONTROL_MASK: u32 = 1 << 2;
const TOGGLE_KEYVAL: u32 = 126; // shortcuts[1] mặc định

fn ibustext(v: &Value<'_>) -> Option<String> {
    match v {
        Value::Structure(s) => {
            if let Some(Value::Str(t)) = s.fields().get(2) {
                return Some(t.to_string());
            }
            s.fields().iter().find_map(ibustext)
        }
        Value::Value(inner) => ibustext(inner),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = pinakey_ibus::address::ibus_address()?;
    println!("IBus address: {addr}");

    let _server = zbus::connection::Builder::address(addr.as_str())?
        .name(TEST_NAME)?
        .serve_at(FACTORY_PATH, Factory::new())?
        .build()
        .await?;
    let conn = zbus::connection::Builder::address(addr.as_str())?
        .build()
        .await?;
    let factory = zbus::Proxy::new(
        &conn,
        TEST_NAME,
        FACTORY_PATH,
        "org.freedesktop.IBus.Factory",
    )
    .await?;
    let engine_path: OwnedObjectPath = factory.call("CreateEngine", &"PinaKey").await?;
    let engine = zbus::Proxy::new(
        &conn,
        TEST_NAME,
        engine_path.as_str().to_owned(),
        "org.freedesktop.IBus.Engine",
    )
    .await?;

    let mut regprops = engine.receive_signal("RegisterProperties").await?;
    let mut commit = engine.receive_signal("CommitText").await?;
    let mut all_ok = true;

    // 1) FocusIn -> RegisterProperties.
    engine.call::<_, _, ()>("FocusIn", &()).await.ok();
    let got_props = tokio::time::timeout(Duration::from_millis(400), regprops.next())
        .await
        .is_ok();
    all_ok &= got_props;
    println!(
        "{} FocusIn -> RegisterProperties (expect có signal)",
        if got_props { "OK  " } else { "FAIL" }
    );

    // 2) Phím tắt tắt tiếng Việt -> phím thường đi qua.
    let toggled: bool = engine
        .call("ProcessKeyEvent", &(TOGGLE_KEYVAL, 0u32, IBUS_CONTROL_MASK))
        .await?;
    let passthrough: bool = engine
        .call("ProcessKeyEvent", &('v' as u32, 0u32, 0u32))
        .await?;
    let ok2 = toggled && !passthrough;
    all_ok &= ok2;
    println!(
        "{} toggle off: combo handled={toggled}, sau đó 'v' handled={passthrough} (expect true & false)",
        if ok2 { "OK  " } else { "FAIL" }
    );

    // 3) Bật lại -> "vieetj" biến đổi thành "việt".
    let _: bool = engine
        .call("ProcessKeyEvent", &(TOGGLE_KEYVAL, 0u32, IBUS_CONTROL_MASK))
        .await?;
    engine.call::<_, _, ()>("Reset", &()).await.ok();
    let mut preedit = engine.receive_signal("UpdatePreeditText").await?;
    for ch in "vieetj".chars() {
        let _: bool = engine
            .call("ProcessKeyEvent", &(ch as u32, 0u32, 0u32))
            .await?;
    }
    let mut last_preedit = None;
    while let Ok(Some(msg)) = tokio::time::timeout(Duration::from_millis(300), preedit.next()).await
    {
        if let Ok((val, _c, _v, _m)) = msg.body().deserialize::<(Value, u32, bool, u32)>() {
            if let Some(t) = ibustext(&val) {
                last_preedit = Some(t);
            }
        }
    }
    let ok3 = last_preedit.as_deref() == Some("việt");
    all_ok &= ok3;
    println!(
        "{} bật lại + \"vieetj\": preedit={:?} (expect \"việt\")",
        if ok3 { "OK  " } else { "FAIL" },
        last_preedit
    );
    let _ = &mut commit;

    // 4) PropertyActivate đổi kiểu gõ -> không sập + đăng ký lại menu.
    let _ = drain_props(&mut regprops).await;
    engine
        .call::<_, _, ()>("PropertyActivate", &("im_VNI", 0u32))
        .await
        .ok();
    let got_refresh = tokio::time::timeout(Duration::from_millis(400), regprops.next())
        .await
        .is_ok();
    all_ok &= got_refresh;
    println!(
        "{} PropertyActivate(im_VNI) -> RegisterProperties lại (expect có signal)",
        if got_refresh { "OK  " } else { "FAIL" }
    );

    println!("\n{}", if all_ok { "ALL OK" } else { "SOME FAILED" });
    if !all_ok {
        std::process::exit(1);
    }
    Ok(())
}

async fn drain_props<T>(stream: &mut T)
where
    T: StreamExt + Unpin,
{
    while tokio::time::timeout(Duration::from_millis(100), stream.next())
        .await
        .is_ok()
    {}
}
