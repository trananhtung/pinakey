//! pinakey entry point — ported from `main.go`.
//!
//! IBus launches the engine with `--ibus` (embedded mode); the installed component XML points at
//! this binary. `--version` prints the version.

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

    // Both the embedded (`--ibus`) and default invocations run the engine over the IBus bus.
    // (The Go standalone mode additionally registers a component descriptor; in production IBus
    // always launches the engine via the installed component XML with `--ibus`.)
    if let Err(e) = run_embedded().await {
        eprintln!("pinakey failed: {e}");
        std::process::exit(1);
    }
}
