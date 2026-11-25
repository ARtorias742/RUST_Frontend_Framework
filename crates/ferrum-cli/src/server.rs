//! Dev server: static + SSE + watcher + rebuild.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// The HMR client injected into index.html (error overlay + CSS swap).
pub const CLIENT_JS: &str = r#"(function () {
  function overlay(msg) {
    var el = document.getElementById('ferrum-error-overlay');
    if (!el) {
      el = document.createElement('div');
      el.id = 'ferrum-error-overlay';
      el.style.cssText = 'position:fixed;left:0;right:0;bottom:0;max-height:40vh;overflow:auto;background:#1f2937;color:#fca5a5;border-top:2px solid #ef4444;font:12px/1.5 monospace;padding:8px;z-index:2147483647;white-space:pre-wrap';
      document.body.appendChild(el);
    }
    el.textContent = msg;
  }
  window.addEventListener('error', function (e) { overlay('Error: ' + e.message); });
  window.addEventListener('unhandledrejection', function (e) { overlay('Unhandled rejection: ' + e.reason); });
  try {
    var es = new EventSource('/__ferrum/events');
    es.onmessage = function (e) {
      var m = JSON.parse(e.data);
      if (m.type === 'reload') { location.reload(); }
      else if (m.type === 'css') {
        var links = document.querySelectorAll('link[rel="stylesheet"]');
        for (var i = 0; i < links.length; i++) {
          if (links[i].getAttribute('href') === m.file) {
            links[i].setAttribute('href', m.file + '?t=' + Date.now());
          }
        }
      } else if (m.type === 'error') { overlay(m.msg); }
    };
  } catch (e) {}
})();"#;

#[derive(Clone)]
struct EventBus {
    clients: Arc<Mutex<Vec<Sender<String>>>>,
}

impl EventBus {
    fn broadcast(&self, msg: String) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain(|tx| tx.send(msg.clone()).is_ok());
    }
}

/// Handle to a running dev server.
pub struct DevServerHandle {
    pub port: u16,
    stop: Arc<AtomicBool>,
}

impl DevServerHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Drop for DevServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Start a dev server with the default build command
/// (`cargo build --target wasm32-unknown-unknown`).
pub fn start(root: impl AsRef<Path>, port: u16) -> std::io::Result<DevServerHandle> {
    start_with_build(
        root,
        port,
        "cargo",
        &["build", "--target", "wasm32-unknown-unknown"],
    )
}

/// Start a dev server with a custom rebuild command (tests).
pub fn start_with_build(
    root: impl AsRef<Path>,
    port: u16,
    program: &str,
    args: &[&str],
) -> std::io::Result<DevServerHandle> {
    let root = root.as_ref().to_path_buf();
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let port = listener.local_addr()?.port();
    let stop = Arc::new(AtomicBool::new(false));
    let events = EventBus {
        clients: Arc::new(Mutex::new(Vec::new())),
    };

    let st = stop.clone();
    let ev = events.clone();
    let r = root.clone();
    thread::spawn(move || accept_loop(listener, r, st, ev));

    let ev2 = events.clone();
    let r2 = root.clone();
    let st2 = stop.clone();
    let program = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    thread::spawn(move || watch_loop(r2, ev2, st2, program, args));

    Ok(DevServerHandle { port, stop })
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

fn accept_loop(listener: TcpListener, root: PathBuf, stop: Arc<AtomicBool>, events: EventBus) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        match stream {
            Ok(mut s) => {
                let root = root.clone();
                let events = events.clone();
                thread::spawn(move || handle_conn(&mut s, &root, &events));
            }
            Err(_) => continue,
        }
    }
}

fn handle_conn(stream: &mut TcpStream, root: &Path, events: &EventBus) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    // Read until end of headers.
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if buf.len() > 64 * 1024 {
                    return;
                }
            }
            Err(_) => return,
        }
    }
    let head = String::from_utf8_lossy(&buf);
    let request_line = head.lines().next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let _method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    match path {
        "/__ferrum/events" => sse_stream(stream, events),
        "/@ferrum/client.js" => respond(stream, "200 OK", "application/javascript", CLIENT_JS),
        "/" | "/index.html" => {
            let html = fs::read_to_string(root.join("index.html")).unwrap_or_default();
            let injected = inject_client(&html);
            respond(stream, "200 OK", "text/html", &injected);
        }
        p => serve_static(stream, root, p),
    }
}

fn inject_client(html: &str) -> String {
    let tag = "<script src=\"/@ferrum/client.js\"></script>";
    if html.contains("</body>") {
        html.replace("</body>", &format!("{}{}", tag, "</body>"))
    } else {
        format!("{}{}", html, tag)
    }
}

fn serve_static(stream: &mut TcpStream, root: &Path, path: &str) {
    let rel = path.strip_prefix('/').unwrap_or(path);
    if rel.contains("..") {
        respond(stream, "403 Forbidden", "text/plain", "forbidden");
        return;
    }
    let file = root.join(rel);
    match fs::read(&file) {
        Ok(bytes) => {
            let mime = mime_for(&file);
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", mime, bytes.len());
            let _ = stream.write_all(&bytes);
        }
        Err(_) => respond(stream, "404 Not Found", "text/plain", "not found"),
    }
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );
}

fn sse_stream(stream: &mut TcpStream, events: &EventBus) {
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.flush();
    let (tx, rx) = channel::<String>();
    events.clients.lock().unwrap().push(tx);

    // Poll for events with a short timeout. Between events, check for a
    // disconnected client *without blocking* (the old blocking read could
    // stall forever and starve the event loop — the cause of the Phase 6
    // test hang). A dead client is pruned on the next broadcast anyway.
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(msg) => {
                if write!(stream, "data: {}\n\n", msg).is_err() {
                    break;
                }
                let _ = stream.flush();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let _ = stream.set_nonblocking(true);
                let mut b = [0u8; 1];
                match stream.read(&mut b) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }
                let _ = stream.set_nonblocking(false);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

// ---------------------------------------------------------------------------
// Watcher + rebuild
// ---------------------------------------------------------------------------

fn watch_loop(
    root: PathBuf,
    events: EventBus,
    stop: Arc<AtomicBool>,
    program: String,
    args: Vec<String>,
) {
    let mut snapshot = scan(&root);
    while !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
        let current = scan(&root);
        if current == snapshot {
            continue;
        }
        // Debounce: wait for quiet.
        thread::sleep(Duration::from_millis(80));
        let changed: Vec<PathBuf> = current
            .iter()
            .filter(|(p, v)| snapshot.get(p.as_path()) != Some(*v))
            .map(|(p, _)| p.clone())
            .chain(
                snapshot
                    .keys()
                    .filter(|p| !current.contains_key(p.as_path()))
                    .cloned(),
            )
            .collect();
        snapshot = scan(&root);
        if changed.is_empty() {
            continue;
        }
        if changed
            .iter()
            .all(|p| p.extension().and_then(|e| e.to_str()) == Some("css"))
        {
            // CSS-only: no reload — the client cache-busts the link.
            for p in &changed {
                let name = p
                    .strip_prefix(&root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_string();
                events.broadcast(format!("{{\"type\":\"css\",\"file\":\"{}\"}}", name));
            }
            continue;
        }
        // Rebuild.
        let start = Instant::now();
        let output = std::process::Command::new(&program)
            .args(&args)
            .current_dir(&root)
            .output();
        let ms = start.elapsed().as_millis();
        match output {
            Ok(o) if o.status.success() => {
                events.broadcast(format!("{{\"type\":\"reload\",\"ms\":{}}}", ms));
            }
            Ok(o) => {
                let msg = String::from_utf8_lossy(&o.stderr);
                let tail: String = msg
                    .chars()
                    .rev()
                    .take(400)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                events.broadcast(format!(
                    "{{\"type\":\"error\",\"msg\":{}}}",
                    json_escape(&tail)
                ));
            }
            Err(e) => {
                events.broadcast(format!(
                    "{{\"type\":\"error\",\"msg\":{}}}",
                    json_escape(&e.to_string())
                ));
            }
        }
    }
}

fn scan(root: &Path) -> std::collections::HashMap<PathBuf, (u64, u64)> {
    let mut out = std::collections::HashMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if matches!(name.as_str(), "target" | "dist" | ".git" | "node_modules") && path.is_dir()
            {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                out.insert(path, (mtime, meta.len()));
            }
        }
    }
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
