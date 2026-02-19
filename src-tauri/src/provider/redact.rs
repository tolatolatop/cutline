use regex::Regex;
use std::sync::LazyLock;

const MAX_LEN: usize = 2048;

static RE_AUTH_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(Authorization:\s*)(.+)").unwrap());
static RE_COOKIE_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(Cookie:\s*)(.+)").unwrap());
static RE_COOKIE_KV: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"((?:sessionid|session_id|sid|token)=)[^\s;]+").unwrap());

pub fn redact(text: &str) -> String {
    let mut out = text.to_string();

    out = RE_AUTH_HEADER
        .replace_all(&out, "${1}<redacted>")
        .to_string();
    out = RE_COOKIE_HEADER
        .replace_all(&out, "${1}<redacted>")
        .to_string();
    out = RE_COOKIE_KV
        .replace_all(&out, "${1}<redacted>")
        .to_string();

    out = redact_url_params(&out);

    if out.len() > MAX_LEN {
        out.truncate(MAX_LEN);
        out.push_str("...<truncated>");
    }
    out
}

fn redact_url_params(text: &str) -> String {
    static RE_URL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(https?://[^\s]+)").unwrap());
    RE_URL
        .replace_all(text, |caps: &regex::Captures| {
            let url = &caps[0];
            if let Some(idx) = url.find('?') {
                url[..idx].to_string()
            } else if let Some(idx) = url.find('#') {
                url[..idx].to_string()
            } else {
                url.to_string()
            }
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_authorization() {
        let input = "Authorization: Bearer sk-abc123xyz";
        assert_eq!(redact(input), "Authorization: <redacted>");
    }

    #[test]
    fn test_redact_cookie() {
        let input = "Cookie: sessionid=abc123; other=val";
        assert_eq!(redact(input), "Cookie: <redacted>");
    }

    #[test]
    fn test_redact_url_query() {
        let input = "Request to https://api.foo.com/v1/gen?token=secret&foo=bar failed";
        assert_eq!(
            redact(input),
            "Request to https://api.foo.com/v1/gen failed"
        );
    }

    #[test]
    fn test_truncate() {
        let long = "a".repeat(3000);
        let result = redact(&long);
        assert!(result.len() <= MAX_LEN + 20);
        assert!(result.ends_with("...<truncated>"));
    }
}
