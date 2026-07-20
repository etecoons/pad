use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use std::time::Duration;
use tokio::fs;

use super::helper::is_path_within_data_dir;
use super::read::PAGE_HISTORY_COOKIE;
use crate::migration::{Notepad, sanitize_filename};
use crate::state::{AppState, NotepadsJson};

pub async fn create_notepad(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    let (new_notepad, id, unique_name) = {
        let _lock = state.notepads_lock.lock().await;

        let file_content = match fs::read_to_string(&state.notepads_file).await {
            Ok(c) => c,
            Err(_) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({ "error": "Error reading notepads file" })),
                )
                    .into_response();
            }
        };

        let mut data: NotepadsJson =
            serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });

        let id = {
            let mut buf = [0u8; 16];
            rand::rng().fill_bytes(&mut buf);
            buf.iter().map(|b| format!("{:02x}", b)).collect::<String>()
        };
        let desired_name = format!("Notepad {}", data.notepads.len() + 1);
        let unique_name = state.generate_unique_name(&desired_name, &data.notepads);

        let new_notepad = Notepad {
            id: id.clone(),
            name: unique_name.clone(),
        };
        data.notepads.push(new_notepad.clone());

        if fs::write(&state.notepads_file, serde_json::to_string(&data).unwrap())
            .await
            .is_err()
        {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "Error updating notepads list" })),
            )
                .into_response();
        }
        (new_notepad, id, unique_name)
    };

    let sanitized = match sanitize_filename(&unique_name) {
        Ok(s) => s,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": format!("Invalid notepad name: {}", e) })),
            )
                .into_response();
        }
    };
    let file_path = state.data_dir.join(format!("{}.txt", sanitized));
    if !is_path_within_data_dir(&file_path, &state.data_dir) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": "Resolved path escapes data directory" })),
        )
            .into_response();
    }
    if fs::write(&file_path, "").await.is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error creating notepad file" })),
        )
            .into_response();
    }

    state.index_notepads().await;

    let secure =
        state.config.server.base_url.starts_with("https") && state.config.node_env == "production";
    let history_age_secs = (state.config.page_history_cookie_age_days * 24 * 3600) as u64;

    let jar = jar.add(
        Cookie::build((PAGE_HISTORY_COOKIE, id))
            .path("/")
            .http_only(true)
            .secure(secure)
            .same_site(SameSite::Strict)
            .max_age(Duration::from_secs(history_age_secs).try_into().unwrap())
            .build(),
    );

    (jar, axum::Json(new_notepad)).into_response()
}
