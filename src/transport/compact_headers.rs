/// Expand compact SIP header names to their full forms per RFC 3261 Section 7.3.3.
///
/// Some SIP implementations (notably Telnyx) use compact header forms like
/// `f:` instead of `From:`. The rsip parser requires full header names,
/// so this function expands them before parsing.
pub fn expand_compact_headers(raw: &str) -> Option<String> {
    // Quick check: scan for any single-char header name followed by colon
    // If none found, return None to avoid unnecessary allocation
    let headers_end = raw.find("\r\n\r\n")?;
    let headers_part = &raw[..headers_end];

    let needs_expansion = headers_part.split("\r\n").skip(1).any(|line| {
        if line.len() >= 2 {
            let first = line.as_bytes()[0];
            let second = line.as_bytes()[1];
            second == b':' && is_compact_header(first.to_ascii_lowercase())
        } else {
            false
        }
    });

    if !needs_expansion {
        return None;
    }

    let body_part = &raw[headers_end..];
    let mut result = String::with_capacity(raw.len() + 128);

    for (i, line) in headers_part.split("\r\n").enumerate() {
        if i > 0 {
            result.push_str("\r\n");
        }

        if i == 0 {
            // Request/status line — keep as is
            result.push_str(line);
            continue;
        }

        if line.len() >= 2 && line.as_bytes()[1] == b':' {
            let ch = line.as_bytes()[0].to_ascii_lowercase();
            if let Some(full_name) = compact_to_full(ch) {
                result.push_str(full_name);
                result.push_str(&line[1..]); // keep colon and value
                continue;
            }
        }

        result.push_str(line);
    }

    result.push_str(body_part);
    Some(result)
}

fn is_compact_header(ch: u8) -> bool {
    matches!(
        ch,
        b'i' | b'm' | b'e' | b'l' | b'c' | b'f' | b's' | b'k' | b't' | b'v'
    )
}

fn compact_to_full(ch: u8) -> Option<&'static str> {
    match ch {
        b'i' => Some("Call-ID"),
        b'm' => Some("Contact"),
        b'e' => Some("Content-Encoding"),
        b'l' => Some("Content-Length"),
        b'c' => Some("Content-Type"),
        b'f' => Some("From"),
        b's' => Some("Subject"),
        b'k' => Some("Supported"),
        b't' => Some("To"),
        b'v' => Some("Via"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_compact_headers() {
        let msg = "INVITE sip:user@example.com SIP/2.0\r\nVia: SIP/2.0/UDP 10.0.0.1\r\nFrom: <sip:a@b>\r\n\r\n";
        assert!(expand_compact_headers(msg).is_none());
    }

    #[test]
    fn test_expands_compact_from() {
        let msg = "INVITE sip:user@example.com SIP/2.0\r\nVia: SIP/2.0/UDP 10.0.0.1\r\nf:<sip:a@b>;tag=abc\r\nt:<sip:c@d>\r\ni:call123\r\nl:0\r\n\r\n";
        let expanded = expand_compact_headers(msg).unwrap();
        assert!(expanded.contains("From:<sip:a@b>"));
        assert!(expanded.contains("To:<sip:c@d>"));
        assert!(expanded.contains("Call-ID:call123"));
        assert!(expanded.contains("Content-Length:0"));
    }

    #[test]
    fn test_preserves_body() {
        let msg = "INVITE sip:u@e SIP/2.0\r\nf:<sip:a@b>\r\nl:5\r\n\r\nhello";
        let expanded = expand_compact_headers(msg).unwrap();
        assert!(expanded.ends_with("\r\n\r\nhello"));
    }

    #[test]
    fn test_mixed_compact_and_full() {
        let msg = "INVITE sip:u@e SIP/2.0\r\nVia: SIP/2.0/UDP 10.0.0.1\r\nf:\"alice\"<sip:a@b>\r\nCSeq: 1 INVITE\r\nl:0\r\n\r\n";
        let expanded = expand_compact_headers(msg).unwrap();
        assert!(expanded.contains("From:\"alice\"<sip:a@b>"));
        assert!(expanded.contains("CSeq: 1 INVITE"));
        assert!(expanded.contains("Content-Length:0"));
    }
}
