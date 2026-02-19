use super::model::{AuthConfig, AuthKind};

pub fn apply_auth(
    builder: reqwest::RequestBuilder,
    auth: &AuthConfig,
    secret: &str,
) -> reqwest::RequestBuilder {
    match auth.kind {
        AuthKind::ApiKey => {
            let header = auth.header.as_deref().unwrap_or("Authorization");
            let prefix = auth.prefix.as_deref().unwrap_or("");
            let value = format!("{}{}", prefix, secret);
            builder.header(header, value)
        }
        AuthKind::SessionCookie => {
            let cookie_name = auth.cookie_name.as_deref().unwrap_or("sessionid");
            let value = format!("{}={}", cookie_name, secret);
            builder.header("Cookie", value)
        }
    }
}
