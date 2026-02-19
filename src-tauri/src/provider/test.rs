use std::time::Instant;

use super::auth::apply_auth;
use super::io::{load_providers, providers_path};
use super::model::TestResult;
use super::redact::redact;
use crate::secrets;

pub async fn run_provider_test(
    app_handle: &tauri::AppHandle,
    provider_name: &str,
    profile_name: &str,
) -> TestResult {
    let path = match providers_path(app_handle) {
        Ok(p) => p,
        Err(e) => return TestResult { ok: false, latency_ms: None, error: Some(e) },
    };
    let file = match load_providers(&path) {
        Ok(f) => f,
        Err(e) => return TestResult { ok: false, latency_ms: None, error: Some(e) },
    };
    let provider = match file.providers.get(provider_name) {
        Some(p) => p,
        None => {
            return TestResult {
                ok: false,
                latency_ms: None,
                error: Some(format!("provider_not_found: {}", provider_name)),
            }
        }
    };
    let profile = match provider.profiles.get(profile_name) {
        Some(p) => p,
        None => {
            return TestResult {
                ok: false,
                latency_ms: None,
                error: Some(format!("profile_not_found: {}", profile_name)),
            }
        }
    };

    let secret = match secrets::get_secret(&profile.credential_ref) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return TestResult {
                ok: false,
                latency_ms: None,
                error: Some("missing_credentials".to_string()),
            }
        }
        Err(e) => return TestResult { ok: false, latency_ms: None, error: Some(e) },
    };

    let test_ep = provider.test.as_ref();
    let method_str = test_ep.map(|t| t.method.as_str()).unwrap_or("GET");
    let path_str = test_ep.map(|t| t.path.as_str()).unwrap_or("/health");

    let url = format!("{}{}", provider.base_url.trim_end_matches('/'), path_str);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(profile.timeout_ms))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return TestResult {
                ok: false,
                latency_ms: None,
                error: Some(redact(&format!("http_client_error: {}", e))),
            }
        }
    };

    let method = match method_str.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "HEAD" => reqwest::Method::HEAD,
        _ => reqwest::Method::GET,
    };

    let builder = client.request(method, &url);
    let builder = apply_auth(builder, &provider.auth, &secret);

    let start = Instant::now();
    match builder.send().await {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as u64;
            let status = resp.status();
            if status.is_success() || status.as_u16() == 204 {
                TestResult {
                    ok: true,
                    latency_ms: Some(latency),
                    error: None,
                }
            } else {
                let body = resp.text().await.unwrap_or_default();
                TestResult {
                    ok: false,
                    latency_ms: Some(latency),
                    error: Some(redact(&format!("http_{}: {}", status.as_u16(), body))),
                }
            }
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as u64;
            let kind = if e.is_timeout() {
                "timeout"
            } else if e.is_connect() {
                "connection_error"
            } else {
                "network_error"
            };
            TestResult {
                ok: false,
                latency_ms: Some(latency),
                error: Some(redact(&format!("{}: {}", kind, e))),
            }
        }
    }
}
