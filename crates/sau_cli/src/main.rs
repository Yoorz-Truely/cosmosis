//! The `sau` command (Cosmósis spec, section 7): project commands without Sau
//! running, headless commands that start Sau, and actions on the open app
//! through its gateway. Not built yet: it only reports its version.

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => println!("sau {version}"),
        _ => {
            eprintln!(
                "sau {version}: no commands are built yet (docs/spec/cosmosis-v0.md, section 7)"
            );
            std::process::exit(2);
        }
    }
}
