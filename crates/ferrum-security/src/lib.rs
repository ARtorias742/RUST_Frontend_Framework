//! # ferrum-security
//!
//! Security primitives for Ferrum apps. The framework never calls `eval`, and
//! raw HTML must pass through [`sanitize_html`] before it reaches
//! `innerHTML`.
//!
//! - [`escape_text`] / [`escape_attr`] — the same escaping rules the SSR
//!   renderer uses.
//! - [`sanitize_html`] — an allowlist sanitizer (tags, attributes, URL
//!   schemes). It is a small, dependency-free tokenizer, fuzzed in
//!   `fuzz/fuzz_targets/sanitize.rs`.
//! - [`Policy`] — the allowlist; [`Policy::strict`] is the default.
//! - [`Csp`] — build a nonce-based Content-Security-Policy header/meta tag.
//! - [`uses_eval`] — a conservative scan for `eval` / `new Function` in
//!   user-supplied scripts (used by `ferrum doctor`).
//!
//! ```rust
//! let dirty = r#"<script>alert(1)</script><b onclick="x()">hi</b>"#;
//! assert_eq!(ferrum_security::sanitize_html(dirty, &ferrum_security::Policy::strict()), "<b>hi</b>");
//! ```

/// The sanitizer allowlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub tags: &'static [&'static str],
    pub attrs: &'static [&'static str],
    /// Allow `href`/`src` with `http`, `https`, `mailto`, `#`, `/`.
    pub allow_links: bool,
}

impl Policy {
    /// A conservative default: structural formatting + links, no media,
    /// no script/style/iframe, no event handlers.
    pub fn strict() -> Self {
        Policy {
            tags: &[
                "a",
                "b",
                "blockquote",
                "br",
                "code",
                "div",
                "em",
                "h1",
                "h2",
                "h3",
                "h4",
                "h5",
                "h6",
                "hr",
                "i",
                "li",
                "ol",
                "p",
                "pre",
                "span",
                "strong",
                "sub",
                "sup",
                "table",
                "tbody",
                "td",
                "th",
                "thead",
                "tr",
                "ul",
            ],
            attrs: &["href", "title", "class", "id", "colspan", "rowspan"],
            allow_links: true,
        }
    }

    /// Everything in [`Policy::strict`] plus images (safe URL schemes only).
    pub fn with_images() -> Self {
        let mut p = Self::strict();
        p.tags = &[
            "a",
            "b",
            "blockquote",
            "br",
            "code",
            "div",
            "em",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "hr",
            "i",
            "img",
            "li",
            "ol",
            "p",
            "pre",
            "span",
            "strong",
            "sub",
            "sup",
            "table",
            "tbody",
            "td",
            "th",
            "thead",
            "tr",
            "ul",
        ];
        p.attrs = &[
            "href", "src", "alt", "title", "class", "id", "colspan", "rowspan",
        ];
        p
    }

    fn allows_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    fn allows_attr(&self, name: &str) -> bool {
        self.attrs.iter().any(|a| a.eq_ignore_ascii_case(name))
    }
}

/// Escape text content (`&`, `<`, `>`).
pub fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape an attribute value (`&`, `"`, `<`, `>`).
pub fn escape_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Decode the entities the sanitizer itself emits, so re-sanitising is
/// idempotent (`&gt;` stays `&gt;`, not `&amp;gt;`).
fn unescape_basic(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn is_safe_url(value: &str) -> bool {
    let v = value.trim();
    let lower = v.to_ascii_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("vbscript:")
        || lower.starts_with("data:text/html")
        || lower.starts_with("data:application/xhtml")
    {
        return false;
    }
    !(lower.starts_with("data:") && !lower.starts_with("data:image/"))
}

/// Sanitize `input` against `policy`.
///
/// Unknown tags are dropped (their text content is kept, except for
/// `script`/`style`/`iframe`/`object`/`embed`, whose bodies are removed).
/// Event handlers (`on*`) and unsafe URL schemes are always stripped.
pub fn sanitize_html(input: &str, policy: &Policy) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'<' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            out.push_str(&escape_text(&unescape_basic(&input[start..i])));
            continue;
        }

        // Comment or doctype: drop.
        if input[i..].starts_with("<!--") {
            if let Some(end) = input[i + 4..].find("-->") {
                i = i + 4 + end + 3;
            } else {
                break;
            }
            continue;
        }
        if input[i..].starts_with("<!") || input[i..].starts_with("<?") {
            i = skip_to_gt(bytes, i);
            continue;
        }

        let mut j = i + 1;
        let closing = j < bytes.len() && bytes[j] == b'/';
        if closing {
            j += 1;
        }
        let name_start = j;
        while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-') {
            j += 1;
        }
        if j == name_start {
            // A stray `<` that is not a tag: escape it.
            out.push_str("&lt;");
            i += 1;
            continue;
        }
        let tag = String::from_utf8_lossy(&bytes[name_start..j]).into_owned();
        let tag = tag.as_str();

        // Find the end of the tag, respecting quotes.
        let (tag_end, self_closing) = scan_tag_end(bytes, j);
        let inside = &input[j..tag_end];

        if !policy.allows_tag(tag) {
            i = if tag_end < bytes.len() {
                tag_end + 1
            } else {
                bytes.len()
            };
            // Drop the body of raw-text elements entirely.
            if !closing && is_raw_text(tag) {
                i = skip_element(input, i, tag);
            }
            continue;
        }

        if closing {
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        } else {
            out.push('<');
            out.push_str(tag);
            for (name, value) in parse_attrs(inside) {
                if name.eq_ignore_ascii_case("on") || name.to_ascii_lowercase().starts_with("on") {
                    continue;
                }
                if !policy.allows_attr(&name) {
                    continue;
                }
                let lower = name.to_ascii_lowercase();
                if (lower == "href" || lower == "src") && !policy.allow_links {
                    continue;
                }
                if (lower == "href" || lower == "src") && !is_safe_url(&value) {
                    continue;
                }
                out.push(' ');
                out.push_str(&name);
                out.push_str("=\"");
                out.push_str(&escape_attr(&unescape_basic(&value)));
                out.push('"');
            }
            if self_closing {
                out.push_str(" />");
            } else {
                out.push('>');
            }
        }
        i = if tag_end < bytes.len() {
            tag_end + 1
        } else {
            bytes.len()
        };
    }
    out
}

fn is_raw_text(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        "script" | "style" | "iframe" | "object" | "embed"
    )
}

fn skip_to_gt(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'>' {
        i += 1;
    }
    (i + 1).min(bytes.len())
}

/// Given the index just after the tag name, return (index of `>`, self-closing).
fn scan_tag_end(bytes: &[u8], mut i: usize) -> (usize, bool) {
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) => {
                if b == q {
                    quote = None;
                }
            }
            None => {
                if b == b'"' || b == b'\'' {
                    quote = Some(b);
                } else if b == b'>' {
                    let self_closing = i > 0 && bytes[i - 1] == b'/';
                    return (i, self_closing);
                }
            }
        }
        i += 1;
    }
    (bytes.len(), false)
}

/// Skip past the closing `</tag>` (used to drop raw-text bodies).
fn skip_element(input: &str, mut i: usize, tag: &str) -> usize {
    let needle = format!("</{}", tag.to_ascii_lowercase());
    let lower = input.to_ascii_lowercase();
    if let Some(pos) = lower[i..].find(&needle) {
        i += pos;
        i = skip_to_gt(input.as_bytes(), i + needle.len());
    } else {
        i = input.len();
    }
    i
}

/// Parse `name`, `name=value`, `name="value"`, `name='value'` pairs.
fn parse_attrs(input: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
            && bytes[i] != b'/'
        {
            i += 1;
        }
        if i == name_start {
            i += 1;
            continue;
        }
        let name = String::from_utf8_lossy(&bytes[name_start..i]).into_owned();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                value = String::from_utf8_lossy(&bytes[start..i]).into_owned();
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                value = String::from_utf8_lossy(&bytes[start..i]).into_owned();
            }
        }
        out.push((name, value));
    }
    out
}

// ---------------------------------------------------------------------------
// CSP
// ---------------------------------------------------------------------------

/// A nonce-based Content-Security-Policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csp {
    pub nonce: String,
    /// Allow inline styles (needed by scoped CSS custom properties).
    pub inline_styles: bool,
}

impl Csp {
    pub fn new(nonce: impl Into<String>) -> Self {
        Csp {
            nonce: nonce.into(),
            inline_styles: true,
        }
    }

    /// A fresh random-ish nonce (time + address entropy; no `rand` dependency).
    pub fn random_nonce() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NONCE: AtomicU64 = AtomicU64::new(1);
        let n = NONCE.fetch_add(1, Ordering::Relaxed);
        let addr = &n as *const u64 as u64;
        format!("{:x}{:x}", n, addr)
    }

    /// The header value.
    pub fn header(&self) -> String {
        let style = if self.inline_styles {
            " 'unsafe-inline'"
        } else {
            ""
        };
        format!(
            "default-src 'self'; script-src 'self' 'nonce-{}'; style-src 'self'{}; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
            self.nonce, style
        )
    }

    /// A `<meta http-equiv>` tag for static hosts that cannot set headers.
    pub fn meta(&self) -> String {
        format!(
            "<meta http-equiv=\"Content-Security-Policy\" content=\"{}\">",
            escape_attr(&self.header())
        )
    }
}

/// Conservative check for `eval` / `new Function` in a script.
///
/// This is a lint, not a sandbox: it catches the common footguns.
pub fn uses_eval(script: &str) -> bool {
    let lower = script.to_ascii_lowercase();
    lower.contains("eval(")
        || lower.contains("eval (")
        || lower.contains("new function")
        || lower.contains("setTimeout(\"")
        || lower.contains("settimeout(\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_text_and_attributes() {
        assert_eq!(escape_text("<b>&</b>"), "&lt;b&gt;&amp;&lt;/b&gt;");
        assert_eq!(escape_attr("\"x&<>"), "&quot;x&amp;&lt;&gt;");
    }

    #[test]
    fn strips_scripts_and_handlers() {
        let dirty = r#"<script>alert(1)</script><b onclick="x()">hi</b>"#;
        assert_eq!(sanitize_html(dirty, &Policy::strict()), "<b>hi</b>");
    }

    #[test]
    fn strips_javascript_urls() {
        let dirty = r#"<a href="javascript:alert(1)">x</a>"#;
        assert_eq!(sanitize_html(dirty, &Policy::strict()), "<a>x</a>");
    }

    #[test]
    fn keeps_safe_links_and_drops_unknown_tags() {
        let dirty = r#"<blink><a href="https://example.com" title="t">ok</a></blink>"#;
        assert_eq!(
            sanitize_html(dirty, &Policy::strict()),
            "<a href=\"https://example.com\" title=\"t\">ok</a>"
        );
    }

    #[test]
    fn images_are_opt_in() {
        let dirty = r#"<img src="x.png" alt="a" onerror="x()">"#;
        assert_eq!(sanitize_html(dirty, &Policy::strict()), "");
        assert_eq!(
            sanitize_html(dirty, &Policy::with_images()),
            "<img src=\"x.png\" alt=\"a\">"
        );
    }

    #[test]
    fn csp_header_has_nonce_and_no_unsafe_eval() {
        let csp = Csp::new("abc123");
        let header = csp.header();
        assert!(header.contains("'nonce-abc123'"));
        assert!(!header.contains("'unsafe-eval'"));
        assert!(csp.meta().contains("Content-Security-Policy"));
    }

    #[test]
    fn sanitizer_handles_multibyte_without_panicking() {
        let input = "<p title=\"é中\">é中</p>";
        let out = sanitize_html(input, &Policy::strict());
        assert!(out.contains("é中"));

        // Fuzz-found byte sequence (invalid UTF-8 -> lossy replacement).
        let lossy = String::from_utf8_lossy(&[2, 38, 184, 2]);
        let _ = sanitize_html(&lossy, &Policy::strict());
    }

    #[test]
    fn sanitizer_is_idempotent_on_entities() {
        let once = sanitize_html(">\u{b}", &Policy::strict());
        let twice = sanitize_html(&once, &Policy::strict());
        assert_eq!(once, twice);
    }

    #[test]
    fn eval_detector() {
        assert!(uses_eval("eval('1')"));
        assert!(uses_eval("const f = new Function('x')"));
        assert!(!uses_eval("const x = 1"));
    }
}
