use axum::{
    extract::{Query, State, ConnectInfo},
    http::{HeaderMap, Uri},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::fs;

use crate::state::AppState;
use crate::utils::{get_client_ip, secure_compare};

pub const COOKIE_NAME: &str = "rustpad_auth";

// Redirect URL validator helper
pub fn is_valid_redirect_url(url: &str) -> bool {
    if url.is_empty() || !url.starts_with('/') || url.starts_with("//") || url.contains('\\') {
        return false;
    }
    let lower = url.to_lowercase();
    if lower.contains("%2f") || lower.contains("%5c") {
        return false;
    }
    true
}

// Authenticated helper
pub fn is_authenticated(jar: &CookieJar, state: &AppState) -> bool {
    let pin = match &state.config.pin {
        Some(p) => p,
        None => return true,
    };
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        secure_compare(cookie.value(), pin)
    } else {
        false
    }
}

// Pin Middleware
pub async fn require_pin(
    jar: CookieJar,
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "Unauthorized" })),
        )
            .into_response();
    }
    next.run(req).await
}

// Root page server
pub async fn serve_root(
    jar: CookieJar,
    State(state): State<AppState>,
    uri: Uri,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        let redirect_param = percent_encoding::utf8_percent_encode(
            &uri.to_string(),
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        return Redirect::temporary(&format!("/login?redirect={}", redirect_param)).into_response();
    }

    match fs::read_to_string(state.data_dir.parent().unwrap().join("public/index.html")).await {
        Ok(html) => ([(axum::http::header::CONTENT_TYPE, "text/html")], html).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error loading index.html: {}", e),
        )
            .into_response(),
    }
}

// Login page server
pub async fn serve_login(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if is_authenticated(&jar, &state) {
        if let Some(redirect) = params.get("redirect") {
            if is_valid_redirect_url(redirect) {
                return Redirect::temporary(redirect).into_response();
            }
        }
        return Redirect::temporary("/").into_response();
    }

    match fs::read_to_string(state.data_dir.parent().unwrap().join("public/login.html")).await {
        Ok(html) => ([(axum::http::header::CONTENT_TYPE, "text/html")], html).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error loading login.html: {}", e),
        )
            .into_response(),
    }
}

// API: Config
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "siteTitle": state.config.site_title,
        "baseUrl": state.config.base_url,
        "version": state.config.version,
        "highlightLanguages": state.config.highlight_languages,
    }))
}

// API: PIN requirement check
pub async fn pin_required(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let ip = get_client_ip(
        &headers,
        addr,
        state.config.trust_proxy,
        &state.config.trusted_proxies,
    );

    let locked = state.is_locked_out(ip).await;
    axum::Json(serde_json::json!({
        "required": state.config.pin.is_some(),
        "length": state.config.pin.as_ref().map_or(0, |p| p.len()),
        "locked": locked
    }))
}

// API: Verify PIN
#[derive(serde::Deserialize)]
pub struct VerifyPinPayload {
    pub pin: String,
}

pub async fn verify_pin(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<VerifyPinPayload>,
) -> impl IntoResponse {
    let pin_req = &state.config.pin;
    if pin_req.is_none() {
        return (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({ "success": true })),
        )
            .into_response();
    }

    let ip = get_client_ip(
        &headers,
        addr,
        state.config.trust_proxy,
        &state.config.trusted_proxies,
    );

    if state.is_locked_out(ip).await {
        let map = state.login_attempts.read().await;
        let last_time = map.get(&ip).map(|a| a.last_attempt).unwrap();
        let lockout_dur = Duration::from_secs(state.config.lockout_time_minutes * 60);
        let time_left = lockout_dur
            .checked_sub(last_time.elapsed())
            .unwrap_or(Duration::ZERO);
        let time_left_min = (time_left.as_secs_f64() / 60.0).ceil() as u64;

        return (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({
                "error": format!("Too many attempts. Please try again in {} minute(s).", time_left_min)
            })),
        )
            .into_response();
    }

    let expected_pin = pin_req.as_ref().unwrap();

    let is_valid_fmt = payload.pin.len() >= 4
        && payload.pin.len() <= 10
        && payload.pin.chars().all(|c| c.is_ascii_digit());

    if !is_valid_fmt {
        state.record_login_attempt(ip).await;
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Invalid PIN format"
            })),
        )
            .into_response();
    }

    if secure_compare(&payload.pin, expected_pin) {
        state.reset_login_attempts(ip).await;

        let cookie_max_age = Duration::from_secs((state.config.cookie_max_age_hours * 3600) as u64);
        let same_site = SameSite::Strict;

        let secure = state.config.base_url.starts_with("https")
            && state.config.node_env == "production";

        let jar = jar.add(
            Cookie::build((COOKIE_NAME, payload.pin))
                .path("/")
                .http_only(true)
                .secure(secure)
                .same_site(same_site)
                .max_age(cookie_max_age.try_into().unwrap())
                .build(),
        );

        (jar, axum::Json(serde_json::json!({ "success": true }))).into_response()
    } else {
        state.record_login_attempt(ip).await;

        let map = state.login_attempts.read().await;
        let attempts_count = map.get(&ip).map(|a| a.count).unwrap_or(0);
        let attempts_left = state.config.max_attempts.saturating_sub(attempts_count);

        (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Invalid PIN",
                "attemptsLeft": attempts_left
            })),
        )
            .into_response()
    }
}
