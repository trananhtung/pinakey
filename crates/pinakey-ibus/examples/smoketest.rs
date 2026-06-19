//! Smoke test end-to-end trên bus IBus thật, không làm ảnh hưởng tới engine nào đã cài.
//!
//! Nó export `Factory` thật của crate này (và `PinaKeyEngine` mà factory tạo ra) dưới một tên bus
//! test riêng, sau đó gọi `ProcessKeyEvent` từ một kết nối client và kiểm tra văn bản commit /
//! preedit thu được. Cách này chạy thử toàn bộ transport: kết nối zbus + tên + object server,
//! factory, thread của engine actor, lõi biến đổi, và serialize IBusText — tất cả trừ việc IBus
//! định tuyến phím gõ thật từ compositor.
//!
//!     cargo run -p pinakey-ibus --example smoketest

use std::time::Duration;

use futures_util::StreamExt;
use zbus::zvariant::{OwnedObjectPath, Value};

use pinakey_ibus::dbus::Factory;

const TEST_NAME: &str = "org.freedesktop.IBus.pinakey.smoketest";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

/// Cả hai signal mang IBusText đều dùng `(sa{sv}sv)`; trường ở chỉ số 2 là văn bản hiển thị.
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

async fn drain<T>(stream: &mut T) -> Vec<String>
where
    T: StreamExt<Item = zbus::Message> + Unpin,
{
    let mut out = Vec::new();
    while let Ok(Some(msg)) = tokio::time::timeout(Duration::from_millis(250), stream.next()).await
    {
        let body = msg.body();
        if let Ok(val) = body.deserialize::<Value>() {
            if let Some(t) = ibustext(&val) {
                out.push(t);
            }
        } else if let Ok((val, _c, _vis, _m)) = body.deserialize::<(Value, u32, bool, u32)>() {
            if let Some(t) = ibustext(&val) {
                out.push(t);
            }
        }
    }
    out
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = pinakey_ibus::address::ibus_address()?;
    println!("IBus address: {addr}");

    // Kết nối server: đăng ký một tên test riêng và export Factory thật.
    let _server = zbus::connection::Builder::address(addr.as_str())?
        .name(TEST_NAME)?
        .serve_at(FACTORY_PATH, Factory::new())?
        .build()
        .await?;
    println!("serving Factory as {TEST_NAME}");

    // Kết nối client trên cùng bus.
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
    println!("CreateEngine -> {}", engine_path.as_str());

    let engine = zbus::Proxy::new(
        &conn,
        TEST_NAME,
        engine_path.as_str().to_owned(),
        "org.freedesktop.IBus.Engine",
    )
    .await?;

    // Thử FocusIn — trên phiên Wayland này nó kích hoạt việc tra cứu window-class qua X11
    // (XWayland); thao tác này không được làm engine crash.
    match engine.call::<_, _, ()>("FocusIn", &()).await {
        Ok(()) => println!("FocusIn: OK (engine survived window-class lookup)"),
        Err(e) => println!("FocusIn: ERROR {e}"),
    }

    let mut commit = engine.receive_signal("CommitText").await?;
    let mut preedit = engine.receive_signal("UpdatePreeditText").await?;

    // (các phím gõ, chuỗi con tiếng Việt mong đợi) — Telex.
    let cases = [
        ("vieetj ", "việt"),
        ("tieengs ", "tiếng"),
        ("chaof ", "chào"),
        ("ddaau ", "đâu"),
        ("nguwowif ", "người"),
    ];

    let mut all_ok = true;
    for (keys, expect) in cases {
        engine.call::<_, _, ()>("Reset", &()).await.ok();
        let _ = drain(&mut commit).await; // xóa phần dư từ Reset/từ trước
        let _ = drain(&mut preedit).await;
        for ch in keys.chars() {
            let keyval = ch as u32;
            let _handled: bool = engine
                .call("ProcessKeyEvent", &(keyval, 0u32, 0u32))
                .await?;
        }
        let committed = drain(&mut commit).await;
        let preedits = drain(&mut preedit).await;
        let seen = format!(
            "{}{}",
            committed.join(""),
            preedits.last().cloned().unwrap_or_default()
        );
        let ok = seen.contains(expect);
        all_ok &= ok;
        println!(
            "{} type {:?}: commit={:?} preedit_last={:?} (expect ⊇ {:?})",
            if ok { "OK  " } else { "FAIL" },
            keys,
            committed,
            preedits.last(),
            expect
        );
    }

    println!("\n{}", if all_ok { "ALL OK" } else { "SOME FAILED" });
    if !all_ok {
        std::process::exit(1);
    }
    Ok(())
}
