//! Fuzz the HTML sanitizer: it must never panic and must always produce
//! output that is safe under the strict policy.
#![no_main]

use ferrum_security::{sanitize_html, Policy};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let out = sanitize_html(&input, &Policy::strict());
    // The sanitizer must never emit a script tag or an inline event handler.
    let lower = out.to_ascii_lowercase();
    assert!(!lower.contains("<script"));
    assert!(!lower.contains("onclick="));
    assert!(!lower.contains("javascript:"));
    // Re-sanitizing is idempotent.
    let again = sanitize_html(&out, &Policy::strict());
    assert_eq!(out, again);
});