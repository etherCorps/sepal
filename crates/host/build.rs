//! Windows gives a program's main thread 1 MB of stack unless the program asks
//! for more when it is linked; macOS and Linux give 8 MB. The JavaScript engine
//! alone may use 4 MB (`src/js/quickjs.rs`), so ask for what the others have.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
        let stack = if msvc {
            "/STACK:8388608"
        } else {
            "-Wl,--stack,8388608"
        };
        println!("cargo:rustc-link-arg-bins={stack}");
    }
}
