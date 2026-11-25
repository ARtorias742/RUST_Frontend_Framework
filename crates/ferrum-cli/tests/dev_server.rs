//! Phase 6 gate (dev server): static serving, client injection, SSE events,
//! CSS hot-swap (no reload) vs rebuild+reload.

use ferrum_dev::start_with_build;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn get(port: u16, path: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", path).unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap();
    String::from_utf8_lossy(&buf).to_string()
}

fn sse_connect(port: u16) -> TcpStream {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET /__ferrum/events HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut buf = [0u8; 256];
    let n = s.read(&mut buf).unwrap();
    let head = String::from_utf8_lossy(&buf[..n]);
    assert!(head.contains("200 OK"), "SSE headers: {head}");
    assert!(head.contains("text/event-stream"), "{head}");
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s
}

fn read_event(s: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 256];
    loop {
        // Never hang the suite: a missing event is a test failure, not a
        // resource-exhausting deadlock.
        let n = s
            .read(&mut tmp)
            .expect("timed out waiting for an SSE event (server did not broadcast)");
        if n == 0 {
            panic!("SSE stream closed before an event arrived");
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(2).any(|w| w == b"\n\n") {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

struct TempDir(std::path::PathBuf);
impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("ferrum-cli-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn serves_index_with_client_injected() {
    let dir = TempDir::new("index");
    fs::write(dir.0.join("index.html"), "<html><body><div id=\"app\"></div></body></html>").unwrap();
    // No-op build command: the rebuild path is exercised separately.
    let handle = start_with_build(&dir.0, 0, "true", &[]).unwrap();
    let body = get(handle.port, "/");
    assert!(body.contains("200 OK"), "{body}");
    assert!(body.contains("<div id=\"app\">"));
    assert!(body.contains("/@ferrum/client.js"), "client not injected: {body}");
}

#[test]
fn serves_client_js() {
    let dir = TempDir::new("client");
    let handle = start_with_build(&dir.0, 0, "true", &[]).unwrap();
    let body = get(handle.port, "/@ferrum/client.js");
    assert!(body.contains("EventSource"), "{body}");
    assert!(body.contains("ferrum-error-overlay"), "error overlay missing");
}

#[test]
fn serves_static_files_with_mime() {
    let dir = TempDir::new("static");
    fs::write(dir.0.join("style.css"), "body { color: red; }").unwrap();
    fs::write(dir.0.join("app.wasm"), b"\0asm").unwrap();
    let handle = start_with_build(&dir.0, 0, "true", &[]).unwrap();
    let css = get(handle.port, "/style.css");
    assert!(css.contains("text/css"), "{css}");
    let wasm = get(handle.port, "/app.wasm");
    assert!(wasm.contains("application/wasm"), "{wasm}");
}

#[test]
fn css_change_pushes_css_event_without_reload() {
    let dir = TempDir::new("css");
    fs::write(dir.0.join("style.css"), "body { color: red; }").unwrap();
    let handle = start_with_build(&dir.0, 0, "true", &[]).unwrap();
    let mut sse = sse_connect(handle.port);
    // Give the watcher a baseline snapshot.
    std::thread::sleep(Duration::from_millis(300));
    fs::write(dir.0.join("style.css"), "body { color: blue; }").unwrap();
    let event = read_event(&mut sse);
    assert!(event.contains("\"type\":\"css\""), "expected css event, got: {event}");
    assert!(event.contains("style.css"), "{event}");
}

#[test]
fn rs_change_triggers_rebuild_and_reload_event() {
    let dir = TempDir::new("reload");
    fs::create_dir_all(dir.0.join("src")).unwrap();
    fs::write(dir.0.join("src").join("lib.rs"), "fn a() {}").unwrap();
    let handle = start_with_build(&dir.0, 0, "true", &[]).unwrap();
    let mut sse = sse_connect(handle.port);
    std::thread::sleep(Duration::from_millis(300));
    fs::write(dir.0.join("src").join("lib.rs"), "fn a() { let _ = 1; }").unwrap();
    let event = read_event(&mut sse);
    assert!(event.contains("\"type\":\"reload\""), "expected reload event, got: {event}");
    assert!(event.contains("\"ms\":"), "rebuild time missing: {event}");
}

#[test]
fn failed_rebuild_pushes_error_event() {
    let dir = TempDir::new("error");
    fs::create_dir_all(dir.0.join("src")).unwrap();
    fs::write(dir.0.join("src").join("lib.rs"), "fn a() {}").unwrap();
    // A build command that fails.
    let handle = start_with_build(&dir.0, 0, "false", &[]).unwrap();
    let mut sse = sse_connect(handle.port);
    std::thread::sleep(Duration::from_millis(300));
    fs::write(dir.0.join("src").join("lib.rs"), "fn a() { compile_error!(\"boom\"); }").unwrap();
    let event = read_event(&mut sse);
    assert!(event.contains("\"type\":\"error\""), "expected error event, got: {event}");
}