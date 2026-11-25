//! `ferrum add` writes source, mod.rs and manifest.

use std::path::PathBuf;
use std::process::Command;

fn ferrum() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ferrum"))
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ferrum-add-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn add_button_writes_source_and_manifest() {
    let dir = temp_dir("button");
    let out = ferrum()
        .args(["add", "button", "--dir", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let button = dir.join("button.rs");
    assert!(button.exists());
    assert!(std::fs::read_to_string(&button)
        .unwrap()
        .contains("pub fn Button"));
    assert!(std::fs::read_to_string(dir.join("mod.rs"))
        .unwrap()
        .contains("pub mod button;"));
    assert!(std::fs::read_to_string(dir.join("manifest.toml"))
        .unwrap()
        .contains("[components.button]"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_shows_templates() {
    let out = ferrum().arg("list").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("button"));
    assert!(stdout.contains("dialog"));
}

#[test]
fn add_refuses_overwrite_without_force() {
    let dir = temp_dir("force");
    let args = ["add", "button", "--dir", dir.to_str().unwrap()];
    assert!(ferrum().args(args).output().unwrap().status.success());
    let out = ferrum().args(args).output().unwrap();
    assert!(!out.status.success());
    let forced = ferrum()
        .args(["add", "button", "--dir", dir.to_str().unwrap(), "--force"])
        .output()
        .unwrap();
    assert!(forced.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}
