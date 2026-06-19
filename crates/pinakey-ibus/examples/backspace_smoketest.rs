//! Smoke test end-to-end cho **chế độ sửa lỗi bằng backspace** trên bus IBus thật.
//!
//! Khác với `smoketest` (kiểm tra luồng Preedit), bài này bật `BackspaceForwarding` rồi kiểm tra
//! engine phát đúng chuỗi `ForwardKeyEvent(BackSpace)` + `CommitText` để biến "vieetj" thành "việt"
//! ngay trên ứng dụng (không dùng vùng preedit). Nó tự ghi một file cấu hình tạm cho một tên engine
//! riêng (không đụng tới cấu hình PinaKey thật) và dọn dẹp khi xong.
//!
//!     cargo run -p pinakey-ibus --example backspace_smoketest

use std::time::Duration;

use futures_util::StreamExt;
use zbus::zvariant::{OwnedObjectPath, Value};

use pinakey_config::{default_cfg, flags, get_config_path, save_config};
use pinakey_ibus::constants::IBUS_RELEASE_MASK;
use pinakey_ibus::dbus::Factory;

const TEST_NAME: &str = "org.freedesktop.IBus.pinakey.bssmoketest";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";
const ENGINE_NAME: &str = "pinakeybstest";
const IBUS_BACKSPACE: u32 = 0xff08;

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
    // 1) Ghi cấu hình tạm bật chế độ BackspaceForwarding cho một engine name riêng.
    let mut cfg = default_cfg();
    cfg.default_input_mode = flags::BACKSPACE_FORWARDING_IM;
    save_config(&cfg, ENGINE_NAME)?;
    let cfg_path = get_config_path(ENGINE_NAME);
    println!(
        "Wrote temp config: {} (BackspaceForwarding)",
        cfg_path.display()
    );

    let result = run(&cfg).await;

    // Dọn file cấu hình tạm dù thành công hay thất bại.
    let _ = std::fs::remove_file(&cfg_path);
    println!("Cleaned up {}", cfg_path.display());

    result
}

async fn run(_cfg: &pinakey_config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let addr = pinakey_ibus::address::ibus_address()?;
    println!("IBus address: {addr}");

    let _server = zbus::connection::Builder::address(addr.as_str())?
        .name(TEST_NAME)?
        .serve_at(FACTORY_PATH, Factory::new())?
        .build()
        .await?;
    println!("serving Factory as {TEST_NAME}");

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
    let engine_path: OwnedObjectPath = factory.call("CreateEngine", &ENGINE_NAME).await?;
    println!("CreateEngine -> {}", engine_path.as_str());

    let engine = zbus::Proxy::new(
        &conn,
        TEST_NAME,
        engine_path.as_str().to_owned(),
        "org.freedesktop.IBus.Engine",
    )
    .await?;

    // Đăng ký match rule cho cả hai signal (giữ stream sống để rule còn hiệu lực), rồi đọc qua một
    // MessageStream duy nhất để giữ đúng thứ tự giữa CommitText và ForwardKeyEvent.
    let _commit_sub = engine.receive_signal("CommitText").await?;
    let _forward_sub = engine.receive_signal("ForwardKeyEvent").await?;
    let mut messages = zbus::MessageStream::from(&conn);

    for ch in "vieetj".chars() {
        let _handled: bool = engine
            .call("ProcessKeyEvent", &(ch as u32, 0u32, 0u32))
            .await?;
    }

    // Mô phỏng màn hình ứng dụng theo đúng thứ tự signal tới: CommitText nối thêm, mỗi lần nhấn
    // (press) BackSpace xóa một ký tự cuối.
    let mut screen: Vec<char> = Vec::new();
    let mut backspaces = 0usize;
    while let Ok(Some(Ok(msg))) =
        tokio::time::timeout(Duration::from_millis(400), messages.next()).await
    {
        let member = msg
            .header()
            .member()
            .map(|m| m.to_string())
            .unwrap_or_default();
        match member.as_str() {
            "CommitText" => {
                if let Ok(val) = msg.body().deserialize::<Value>() {
                    if let Some(t) = ibustext(&val) {
                        screen.extend(t.chars());
                    }
                }
            }
            "ForwardKeyEvent" => {
                if let Ok((keyval, _code, state)) = msg.body().deserialize::<(u32, u32, u32)>() {
                    if keyval == IBUS_BACKSPACE && state & IBUS_RELEASE_MASK == 0 {
                        backspaces += 1;
                        screen.pop();
                    }
                }
            }
            _ => {}
        }
    }

    let seen: String = screen.into_iter().collect();
    println!("forwarded {backspaces} backspace(s); final screen = {seen:?} (expect \"việt\")");
    if seen == "việt" {
        println!("\nALL OK");
        Ok(())
    } else {
        println!("\nFAILED");
        std::process::exit(1);
    }
}
