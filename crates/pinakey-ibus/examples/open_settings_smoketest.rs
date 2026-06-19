//! Smoke test: kích hoạt mục menu "Mở bảng thiết lập…" qua D-Bus và xác nhận engine spawn binary
//! `pinakey-settings`. Sau khi chạy, kiểm tra (bằng `pgrep`) tiến trình settings đã xuất hiện.
//!
//!     cargo run -p pinakey-ibus --example open_settings_smoketest

use zbus::zvariant::OwnedObjectPath;

use pinakey_ibus::dbus::Factory;

const TEST_NAME: &str = "org.freedesktop.IBus.pinakey.opensettings";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = pinakey_ibus::address::ibus_address()?;
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

    println!("Gọi PropertyActivate(open_settings) ...");
    engine
        .call::<_, _, ()>("PropertyActivate", &("open_settings", 0u32))
        .await
        .ok();
    println!("Đã gọi. Engine sẽ spawn 'pinakey-settings'. Giữ tiến trình sống 3s để kiểm tra.");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    Ok(())
}
