use md5::{Digest, Md5};
use rand::Rng;

use super::constants::{APP_VERSION, PLATFORM_CODE, SIGN_PREFIX, SIGN_SUFFIX};
use super::now_secs;

fn random_digits(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len).map(|_| (b'0' + rng.gen_range(0..10)) as char).collect()
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill(&mut buf[..]);
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

fn random_alphanumeric(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn random_base64url(bytes: usize) -> String {
    use base64::Engine;
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill(&mut buf[..]);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&buf)
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

/// 根据 sessionid token 生成完整 Cookie 字符串。
pub fn generate_cookie(token: &str) -> String {
    let ts = now_secs();
    let install_id = random_digits(16);
    let ttreq = format!("1${}", random_alphanumeric(40));
    let csrf_token = random_hex(16);
    let csrf_token_default = random_hex(16);
    let n_mh = random_base64url(32);
    let uid_tt = random_hex(16);
    let uid_tt_ss = random_hex(16);

    let date_str = chrono::Utc::now()
        .format("%a+%d+%b+%Y+%H:%M:%S+GMT")
        .to_string();
    let sid_guard_raw = format!("{}|{}|5183999|{}", token, ts, date_str);
    let sid_guard = percent_encode(&sid_guard_raw);

    let parts = [
        format!("sessionid={}", token),
        format!("sessionid_ss={}", token),
        format!("sid_tt={}", token),
        format!("sid_guard={}", sid_guard),
        format!("install_id={}", install_id),
        format!("ttreq={}", ttreq),
        format!("passport_csrf_token={}", csrf_token),
        format!("passport_csrf_token_default={}", csrf_token_default),
        "is_staff_user=false".to_string(),
        format!("n_mh={}", n_mh),
        format!("uid_tt={}", uid_tt),
        format!("uid_tt_ss={}", uid_tt_ss),
        "sid_ucp_v1=placeholder".to_string(),
        "ssid_ucp_v1=placeholder".to_string(),
    ];

    parts.join("; ")
}

/// 生成内部 API 的 sign 值。
///
/// sign = MD5("9e2c|{uri_last_7}|7|8.4.0|{device_time}||11ac")
pub fn generate_sign(uri: &str, device_time: u64) -> String {
    let uri_bytes = uri.as_bytes();
    let start = if uri_bytes.len() > 7 {
        uri_bytes.len() - 7
    } else {
        0
    };
    let uri_suffix = &uri[start..];

    let raw = format!(
        "{}|{}|{}|{}|{}||{}",
        SIGN_PREFIX, uri_suffix, PLATFORM_CODE, APP_VERSION, device_time, SIGN_SUFFIX
    );

    let mut hasher = Md5::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_is_32_char_hex() {
        let sign = generate_sign("/mweb/v1/aigc_draft/generate", 1700000000);
        assert_eq!(sign.len(), 32);
        assert!(sign.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_is_deterministic() {
        let a = generate_sign("/mweb/v1/aigc_draft/generate", 1700000000);
        let b = generate_sign("/mweb/v1/aigc_draft/generate", 1700000000);
        assert_eq!(a, b);
    }

    #[test]
    fn sign_changes_with_uri() {
        let a = generate_sign("/mweb/v1/aigc_draft/generate", 1700000000);
        let b = generate_sign("/mweb/v1/get_history_by_ids", 1700000000);
        assert_ne!(a, b);
    }

    #[test]
    fn sign_changes_with_time() {
        let a = generate_sign("/mweb/v1/aigc_draft/generate", 1700000000);
        let b = generate_sign("/mweb/v1/aigc_draft/generate", 1700000001);
        assert_ne!(a, b);
    }

    #[test]
    fn sign_matches_python_formula() {
        // Python: MD5("9e2c|enerate|7|8.4.0|1700000000||11ac")
        let sign = generate_sign("/mweb/v1/aigc_draft/generate", 1700000000);
        let raw = "9e2c|enerate|7|8.4.0|1700000000||11ac";
        let mut hasher = Md5::new();
        hasher.update(raw.as_bytes());
        let expected = format!("{:x}", hasher.finalize());
        assert_eq!(sign, expected);
    }

    #[test]
    fn sign_short_uri() {
        // URI shorter than 7 chars: use the whole URI
        let sign = generate_sign("/ab", 1000);
        let raw = "9e2c|/ab|7|8.4.0|1000||11ac";
        let mut hasher = Md5::new();
        hasher.update(raw.as_bytes());
        let expected = format!("{:x}", hasher.finalize());
        assert_eq!(sign, expected);
    }

    #[test]
    fn cookie_contains_all_required_fields() {
        let cookie = generate_cookie("abc123");
        let required = [
            "sessionid=abc123",
            "sessionid_ss=abc123",
            "sid_tt=abc123",
            "sid_guard=",
            "install_id=",
            "ttreq=",
            "passport_csrf_token=",
            "passport_csrf_token_default=",
            "is_staff_user=false",
            "n_mh=",
            "uid_tt=",
            "uid_tt_ss=",
            "sid_ucp_v1=",
            "ssid_ucp_v1=",
        ];
        for field in required {
            assert!(cookie.contains(field), "Cookie missing field: {}", field);
        }
    }

    #[test]
    fn cookie_semicolon_separated() {
        let cookie = generate_cookie("tok");
        let parts: Vec<&str> = cookie.split("; ").collect();
        assert_eq!(parts.len(), 14, "Cookie should have 14 parts, got {}", parts.len());
    }

    #[test]
    fn cookie_sid_guard_is_percent_encoded() {
        let cookie = generate_cookie("mytoken");
        // sid_guard contains | which should be percent-encoded as %7C
        assert!(cookie.contains("sid_guard=mytoken%7C"), "sid_guard should be percent-encoded");
    }

    #[test]
    fn cookie_install_id_is_digits() {
        let cookie = generate_cookie("tok");
        let install_part = cookie
            .split("; ")
            .find(|p| p.starts_with("install_id="))
            .unwrap();
        let val = install_part.strip_prefix("install_id=").unwrap();
        assert_eq!(val.len(), 16);
        assert!(val.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn cookie_ttreq_format() {
        let cookie = generate_cookie("tok");
        let ttreq_part = cookie
            .split("; ")
            .find(|p| p.starts_with("ttreq="))
            .unwrap();
        let val = ttreq_part.strip_prefix("ttreq=").unwrap();
        assert!(val.starts_with("1$"), "ttreq should start with '1$'");
        assert_eq!(val.len(), 2 + 40, "ttreq should be '1$' + 40 chars");
    }

    #[test]
    fn percent_encode_basic() {
        assert_eq!(percent_encode("abc"), "abc");
        assert_eq!(percent_encode("a|b"), "a%7Cb");
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn cookie_randomness_differs_between_calls() {
        let a = generate_cookie("same_token");
        let b = generate_cookie("same_token");
        // sessionid fields will be identical but random fields (install_id, ttreq, etc.) should differ
        assert_ne!(a, b, "two cookie generations should produce different random fields");
    }

    #[test]
    fn cookie_csrf_tokens_are_hex() {
        let cookie = generate_cookie("tok");
        let parts: Vec<&str> = cookie.split("; ").collect();
        for part in &parts {
            if part.starts_with("passport_csrf_token=") && !part.contains("default") {
                let val = part.strip_prefix("passport_csrf_token=").unwrap();
                assert_eq!(val.len(), 32, "csrf_token should be 32 hex chars");
                assert!(val.chars().all(|c| c.is_ascii_hexdigit()));
            }
            if part.starts_with("passport_csrf_token_default=") {
                let val = part.strip_prefix("passport_csrf_token_default=").unwrap();
                assert_eq!(val.len(), 32, "csrf_token_default should be 32 hex chars");
                assert!(val.chars().all(|c| c.is_ascii_hexdigit()));
            }
        }
    }
}
