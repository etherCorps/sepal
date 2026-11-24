//! `calyx --version`: the crate's version, which scripts/check-versions.js
//! keeps equal to the npm packages', so a binary says which release it came in.

#[test]
fn version_is_the_crates() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_calyx"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("calyx {}\n", env!("CARGO_PKG_VERSION"))
    );
}
