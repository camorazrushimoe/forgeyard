/// Classify a provider/LLM failure from bounded Pi stderr.
/// Returns Some(class) when a failure is detected.
pub fn classify_stderr(stderr: &str) -> Option<String> {
    let t = stderr.to_ascii_lowercase();
    if t.trim().is_empty() {
        return None;
    }
    if looks_like_auth(&t) {
        return Some("llm_http_401".into());
    }
    if t.contains("403") && (t.contains("forbidden") || t.contains("http")) {
        return Some("llm_http_403".into());
    }
    if t.contains("429") || t.contains("rate limit") {
        return Some("llm_http_429".into());
    }
    if t.contains("502") || t.contains("503") || t.contains("504") || t.contains("502 bad gateway") {
        return Some("llm_http_5xx".into());
    }
    if looks_like_connect(&t) {
        return Some("llm_connect".into());
    }
    if looks_like_protocol(&t) {
        return Some("llm_protocol".into());
    }
    None
}

fn looks_like_auth(t: &str) -> bool {
    t.contains("401")
        || t.contains("unauthorized")
        || t.contains("invalid api key")
        || t.contains("invalid_api_key")
        || t.contains("incorrect api key")
}

fn looks_like_connect(t: &str) -> bool {
    t.contains("connection refused")
        || t.contains("econnrefused")
        || t.contains("connect: ")
        || t.contains("could not connect")
        || t.contains("connection reset")
        || t.contains("network is unreachable")
        || t.contains("no such host")
        || t.contains("name or service not known")
        || t.contains("timed out")
        || t.contains("timeout")
        || t.contains("dns")
}

fn looks_like_protocol(t: &str) -> bool {
    t.contains("unexpected eof")
        || t.contains("invalid json")
        || t.contains("json parse")
        || t.contains("protocol error")
        || t.contains("unsupported model")
        || t.contains("bad request")
        || t.contains("decode error")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_is_none() {
        assert_eq!(classify_stderr("   "), None);
    }
    #[test]
    fn connect_and_auth() {
        assert_eq!(classify_stderr("dial tcp: connection refused"), Some("llm_connect".into()));
        assert_eq!(classify_stderr("HTTP 401 Unauthorized"), Some("llm_http_401".into()));
        assert_eq!(classify_stderr("invalid JSON from server"), Some("llm_protocol".into()));
    }
}
