//#![deny(unsafe_code)]
//! HTTP admission, header validation, and response framing for maintenance_ui utility.

use anyhow::{Context, Result, bail};
use serde_json::json;
use std::io::Read;
use tiny_http::{Header, Request, Response};

/// Standard security headers applied to all responses.
const SECURITY_HEADERS: [(&str, &str); 5] = [
    ("Cache-Control", "no-store"),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "no-referrer"),
    (
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
    ),
    ("Connection", "close"),
];

/// Extracts an HTTP header value by name (case-insensitive).
pub fn get_header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
}

/// Checks whether an `Expect` header is present (e.g., `Expect: 100-continue`).
pub fn has_expect_header(request: &Request) -> bool {
    get_header(request, "Expect").is_some()
}

/// Checks whether an unsupported `Transfer-Encoding` header is present.
pub fn has_unsupported_transfer_encoding(request: &Request) -> bool {
    get_header(request, "Transfer-Encoding").is_some()
}

/// Constant-time byte comparison for credential checks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Validates Host, Token, and Origin headers.
pub fn authorize(request: &Request, host: &str, origin: &str, token: &str) -> bool {
    let header_token = get_header(request, "X-Devtools-Token");
    let token_valid = header_token
        .map(|v| constant_time_eq(v.as_bytes(), token.as_bytes()))
        .unwrap_or(false);
    get_header(request, "Host") == Some(host)
        && token_valid
        && get_header(request, "Origin").is_none_or(|value| value == origin)
}

/// Reads a bounded JSON body from a request, preventing unbounded stalls or payload flooding.
pub fn read_bounded_json_body(request: &mut Request, max_bytes: usize) -> Result<String> {
    if get_header(request, "Content-Type") != Some("application/json") {
        bail!("A JSON payload with Content-Type: application/json is required");
    }

    if let Some(length) = request.body_length() {
        if length > max_bytes {
            bail!("Payload length {length} exceeds max bound of {max_bytes} bytes");
        }
    }

    let mut body = String::new();
    request
        .as_reader()
        .take((max_bytes + 1) as u64)
        .read_to_string(&mut body)
        .context("Failed reading request body")?;

    if body.len() > max_bytes {
        bail!("Payload exceeded max bound of {max_bytes} bytes");
    }

    Ok(body)
}

/// Safely sends an HTTP response. Returns Ok(()) even if client disconnected to keep the loop resilient.
pub fn respond(
    request: Request,
    status: u16,
    content_type: &str,
    body_content: String,
) -> Result<()> {
    let mut response = Response::from_string(body_content).with_status_code(status);

    let content_type_header = Header::from_bytes("Content-Type", content_type)
        .map_err(|_| anyhow::anyhow!("Invalid Content-Type header"))?;
    response.add_header(content_type_header);

    for (name, value) in SECURITY_HEADERS {
        if let Ok(hdr) = Header::from_bytes(name, value) {
            response.add_header(hdr);
        }
    }

    // Attempt to send response; if client disconnected, swallow error cleanly.
    let _ = request.respond(response);
    Ok(())
}

/// Safely rejects requests containing unsupported `Expect` headers (e.g. `Expect: 100-continue`).
pub fn reject_expect_header(request: Request) -> Result<()> {
    respond(
        request,
        417,
        "application/json",
        json!({"error": "Expect header (e.g. 100-continue) is not supported by this server."})
            .to_string(),
    )
}

/// Safely rejects requests containing unsupported framing/transfer encodings.
pub fn reject_unsupported_framing(request: Request) -> Result<()> {
    respond(
        request,
        400,
        "application/json",
        json!({"error": "Unsupported Transfer-Encoding framing."}).to_string(),
    )
}
