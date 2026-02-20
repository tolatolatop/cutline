use std::time::Duration;

use rand::Rng;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

use super::auth::{generate_cookie, generate_sign};
use super::constants::*;
use super::now_secs;

pub struct JimengClient {
    base_url: String,
    cookie: String,
    web_id: String,
    http: reqwest::Client,
}

impl JimengClient {
    pub fn new(token: &str, base_url: Option<&str>, timeout_secs: u64) -> Result<Self, String> {
        let base = base_url
            .unwrap_or(BASE_URL)
            .trim_end_matches('/')
            .to_string();

        let cookie = generate_cookie(token);

        let mut rng = rand::thread_rng();
        let web_id: u64 = rng.gen_range(1_000_000_000_000_000_000..10_000_000_000_000_000_000);
        let web_id = web_id.to_string();

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            base_url: base,
            cookie,
            web_id,
            http,
        })
    }

    pub(crate) fn common_headers(&self, uri: &str) -> HeaderMap {
        let device_time = now_secs();
        let sign = generate_sign(uri, device_time);

        let pairs: Vec<(&str, String)> = vec![
            ("Accept", "application/json, text/plain, */*".into()),
            ("Accept-Language", "zh-CN,zh;q=0.9".into()),
            ("Cache-Control", "no-cache".into()),
            ("Content-Type", "application/json".into()),
            ("Appid", APP_ID.into()),
            ("Appvr", APP_VERSION.into()),
            ("device-time", device_time.to_string()),
            ("sign-ver", "1".into()),
            ("sign", sign),
            ("loc", "cn".into()),
            ("app-sdk-version", APP_SDK_VERSION.into()),
            ("tdid", String::new()),
            ("lan", "zh-Hans".into()),
            ("Origin", BASE_URL.into()),
            ("Pragma", "no-cache".into()),
            ("Priority", "u=1, i".into()),
            ("Referer", BASE_URL.into()),
            ("Pf", PLATFORM_CODE.into()),
            (
                "Sec-Ch-Ua",
                r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#.into(),
            ),
            ("Sec-Ch-Ua-Mobile", "?0".into()),
            ("Sec-Ch-Ua-Platform", r#""Windows""#.into()),
            ("Sec-Fetch-Dest", "empty".into()),
            ("Sec-Fetch-Mode", "cors".into()),
            ("Sec-Fetch-Site", "same-origin".into()),
            (
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".into(),
            ),
            ("Cookie", self.cookie.clone()),
        ];

        let mut headers = HeaderMap::new();
        for (k, v) in pairs {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(k.as_bytes()),
                HeaderValue::from_str(&v),
            ) {
                headers.insert(name, val);
            }
        }
        headers
    }

    pub(crate) fn common_params(&self, model_name: &str, has_ref_image: bool) -> Vec<(String, String)> {
        let babi_param = if has_ref_image {
            serde_json::json!({
                "scenario": "image_video_generation",
                "feature_key": "to_image_referenceimage_generate",
                "feature_entrance": "to_image",
                "feature_entrance_detail": "to_image-referenceimage-byte_edit"
            })
        } else {
            let detail = if model_name.is_empty() {
                "to_image".to_string()
            } else {
                format!("to_image-{}", model_name)
            };
            serde_json::json!({
                "scenario": "image_video_generation",
                "feature_key": "aigc_to_image",
                "feature_entrance": "to_image",
                "feature_entrance_detail": detail
            })
        };

        vec![
            ("aid".into(), APP_ID.into()),
            ("device_platform".into(), "web".into()),
            ("region".into(), "cn".into()),
            ("webId".into(), self.web_id.clone()),
            ("web_version".into(), WEB_VERSION.into()),
            ("da_version".into(), DA_VERSION.into()),
            ("aigc_features".into(), AIGC_FEATURES.into()),
            ("babi_param".into(), babi_param.to_string()),
        ]
    }

    /// 发送 POST 请求到即梦内部 API。
    pub async fn post(
        &self,
        path: &str,
        body: &Value,
        model_name: &str,
        has_ref_image: bool,
        extra_headers: Option<&[(&str, &str)]>,
    ) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let mut headers = self.common_headers(path);

        if let Some(extras) = extra_headers {
            for (k, v) in extras {
                if let (Ok(name), Ok(val)) = (
                    HeaderName::from_bytes(k.as_bytes()),
                    HeaderValue::from_str(v),
                ) {
                    headers.insert(name, val);
                }
            }
        }

        let params = self.common_params(model_name, has_ref_image);

        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .query(&params)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if !status.is_success() {
            return Err(format!("HTTP {}: {}", status, text));
        }

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse JSON response: {} (body: {})", e, &text[..text.len().min(200)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client() -> JimengClient {
        JimengClient::new("test_token", None, 30).unwrap()
    }

    #[test]
    fn client_creates_successfully() {
        let client = make_client();
        assert_eq!(client.base_url, BASE_URL);
        assert!(!client.cookie.is_empty());
        assert!(!client.web_id.is_empty());
    }

    #[test]
    fn client_custom_base_url() {
        let client = JimengClient::new("tok", Some("https://custom.example.com/"), 10).unwrap();
        assert_eq!(client.base_url, "https://custom.example.com");
    }

    #[test]
    fn client_web_id_is_19_digit_number() {
        let client = make_client();
        assert!(client.web_id.len() == 19, "web_id should be 19 digits, got {}", client.web_id.len());
        assert!(client.web_id.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn headers_contain_all_required_keys() {
        let client = make_client();
        let headers = client.common_headers("/mweb/v1/aigc_draft/generate");

        let required_keys = [
            "accept", "content-type", "appid", "appvr", "device-time",
            "sign-ver", "sign", "loc", "app-sdk-version", "origin",
            "referer", "pf", "user-agent", "cookie",
        ];

        for key in required_keys {
            assert!(
                headers.get(key).is_some(),
                "Missing required header: {}",
                key
            );
        }
    }

    #[test]
    fn headers_appid_matches_constant() {
        let client = make_client();
        let headers = client.common_headers("/test");
        assert_eq!(headers.get("appid").unwrap().to_str().unwrap(), APP_ID);
    }

    #[test]
    fn headers_appvr_matches_constant() {
        let client = make_client();
        let headers = client.common_headers("/test");
        assert_eq!(headers.get("appvr").unwrap().to_str().unwrap(), APP_VERSION);
    }

    #[test]
    fn headers_sign_is_32_hex() {
        let client = make_client();
        let headers = client.common_headers("/mweb/v1/test");
        let sign = headers.get("sign").unwrap().to_str().unwrap();
        assert_eq!(sign.len(), 32);
        assert!(sign.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn headers_device_time_is_numeric() {
        let client = make_client();
        let headers = client.common_headers("/test");
        let dt = headers.get("device-time").unwrap().to_str().unwrap();
        assert!(dt.parse::<u64>().is_ok(), "device-time should be numeric");
    }

    #[test]
    fn headers_cookie_contains_session() {
        let client = JimengClient::new("my_session_abc", None, 30).unwrap();
        let headers = client.common_headers("/test");
        let cookie = headers.get("cookie").unwrap().to_str().unwrap();
        assert!(cookie.contains("sessionid=my_session_abc"));
    }

    #[test]
    fn params_default_keys() {
        let client = make_client();
        let params = client.common_params("", false);
        let keys: Vec<&str> = params.iter().map(|(k, _)| k.as_str()).collect();

        assert!(keys.contains(&"aid"));
        assert!(keys.contains(&"device_platform"));
        assert!(keys.contains(&"region"));
        assert!(keys.contains(&"webId"));
        assert!(keys.contains(&"web_version"));
        assert!(keys.contains(&"da_version"));
        assert!(keys.contains(&"aigc_features"));
        assert!(keys.contains(&"babi_param"));
    }

    #[test]
    fn params_aid_matches_constant() {
        let client = make_client();
        let params = client.common_params("", false);
        let aid = params.iter().find(|(k, _)| k == "aid").unwrap();
        assert_eq!(aid.1, APP_ID);
    }

    #[test]
    fn params_babi_param_is_valid_json() {
        let client = make_client();
        let params = client.common_params("test_model", false);
        let babi = params.iter().find(|(k, _)| k == "babi_param").unwrap();
        let v: serde_json::Value = serde_json::from_str(&babi.1).expect("babi_param should be valid JSON");
        assert_eq!(v["scenario"], "image_video_generation");
        assert_eq!(v["feature_key"], "aigc_to_image");
    }

    #[test]
    fn params_babi_param_ref_image() {
        let client = make_client();
        let params = client.common_params("", true);
        let babi = params.iter().find(|(k, _)| k == "babi_param").unwrap();
        let v: serde_json::Value = serde_json::from_str(&babi.1).unwrap();
        assert_eq!(v["feature_key"], "to_image_referenceimage_generate");
    }

    #[test]
    fn params_babi_param_includes_model_name() {
        let client = make_client();
        let params = client.common_params("jimeng-4.5", false);
        let babi = params.iter().find(|(k, _)| k == "babi_param").unwrap();
        let v: serde_json::Value = serde_json::from_str(&babi.1).unwrap();
        assert_eq!(v["feature_entrance_detail"], "to_image-jimeng-4.5");
    }

    #[test]
    fn params_web_id_matches_client() {
        let client = make_client();
        let params = client.common_params("", false);
        let web_id = params.iter().find(|(k, _)| k == "webId").unwrap();
        assert_eq!(web_id.1, client.web_id);
    }
}
