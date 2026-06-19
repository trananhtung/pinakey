//! Smoke test end-to-end cho **bảng tra cứu emoji + hexadecimal** trên bus IBus thật.
//!
//! Chế độ emoji được kích hoạt bằng `:` ở đầu từ, độc lập với input mode. Bài này kiểm tra:
//!  - gõ `:grin` phát signal `UpdateLookupTable` (có ứng viên emoji);
//!  - gõ `:u+2764` rồi Space commit ra ký tự `❤` (mã hex Unicode).
//!
//!     cargo run -p pinakey-ibus --example emoji_smoketest

use std::time::Duration;

use futures_util::StreamExt;
use zbus::zvariant::{OwnedObjectPath, Value};

use pinakey_ibus::dbus::Factory;

const TEST_NAME: &str = "org.freedesktop.IBus.pinakey.emojismoketest";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";
const IBUS_SPACE: u32 = 0x020;

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
    let engine_path: OwnedObjectPath = factory.call("CreateEngine", &"PinaKey").await?;
    let engine = zbus::Proxy::new(
        &conn,
        TEST_NAME,
        engine_path.as_str().to_owned(),
        "org.freedesktop.IBus.Engine",
    )
    .await?;

    let mut commit = engine.receive_signal("CommitText").await?;
    let mut lookup = engine.receive_signal("UpdateLookupTable").await?;

    let mut all_ok = true;

    // 1) ":grin" -> có UpdateLookupTable.
    for ch in ":grin".chars() {
        let _: bool = engine
            .call("ProcessKeyEvent", &(ch as u32, 0u32, 0u32))
            .await?;
    }
    let mut got_lookup = 0;
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(300), lookup.next()).await {
        got_lookup += 1;
    }
    let ok1 = got_lookup > 0;
    all_ok &= ok1;
    println!(
        "{} type \":grin\": {} UpdateLookupTable signal(s) (expect > 0)",
        if ok1 { "OK  " } else { "FAIL" },
        got_lookup
    );

    engine.call::<_, _, ()>("Reset", &()).await.ok();

    // 2) ":u+2764" + Space -> commit "❤".
    for ch in ":u+2764".chars() {
        let _: bool = engine
            .call("ProcessKeyEvent", &(ch as u32, 0u32, 0u32))
            .await?;
    }
    let _: bool = engine
        .call("ProcessKeyEvent", &(IBUS_SPACE, 0u32, 0u32))
        .await?;

    let mut committed = Vec::new();
    while let Ok(Some(msg)) = tokio::time::timeout(Duration::from_millis(300), commit.next()).await
    {
        if let Ok(val) = msg.body().deserialize::<Value>() {
            if let Some(t) = ibustext(&val) {
                committed.push(t);
            }
        }
    }
    let ok2 = committed.iter().any(|c| c == "❤");
    all_ok &= ok2;
    println!(
        "{} type \":u+2764 \": commit={:?} (expect ⊇ \"❤\")",
        if ok2 { "OK  " } else { "FAIL" },
        committed
    );

    println!("\n{}", if all_ok { "ALL OK" } else { "SOME FAILED" });
    if !all_ok {
        std::process::exit(1);
    }
    Ok(())
}
