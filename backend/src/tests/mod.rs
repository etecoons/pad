use super::*;

#[test]
fn test_fuzzy_match_subsequence() {
    let score_exact = search::fuzzy_match_subsequence("log", "log");
    assert!(score_exact.is_some());

    let score_sub = search::fuzzy_match_subsequence("rapids", "rpd");
    assert!(score_sub.is_some());

    let score_none = search::fuzzy_match_subsequence("log", "xyz");
    assert!(score_none.is_none());
}

#[tokio::test]
async fn test_authenticated_no_pin_required() {
    use axum::http::HeaderMap;
    use axum_extra::extract::cookie::CookieJar;
    use std::collections::{HashMap, HashSet};
    use tokio::sync::RwLock;

    let config = AppConfig::load_from_env(4402);
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });
    let jar = CookieJar::new();
    let headers = HeaderMap::new();

    assert!(routes::auth::is_authenticated(&jar, &state, &headers).await);
}

#[tokio::test]
async fn test_authenticated_with_valid_header_pin() {
    use axum::http::HeaderMap;
    use axum_extra::extract::cookie::CookieJar;
    use std::collections::{HashMap, HashSet};
    use tokio::sync::RwLock;

    let mut config = AppConfig::load_from_env(4402);
    config.server.pin = Some("1234".to_string());
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });
    let jar = CookieJar::new();
    let mut headers = HeaderMap::new();
    headers.insert("x-pin", "1234".parse().unwrap());

    assert!(routes::auth::is_authenticated(&jar, &state, &headers).await);

    let mut invalid_headers = HeaderMap::new();
    invalid_headers.insert("x-pin", "9999".parse().unwrap());
    assert!(!routes::auth::is_authenticated(&jar, &state, &invalid_headers).await);
}

#[tokio::test]
async fn test_rate_limiter_budget_management() {
    use std::collections::{HashMap, HashSet};
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::sync::RwLock;

    let config = AppConfig::load_from_env(4402);
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });
    let test_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    // First request should pass rate limit
    assert!(state.check_rate_limit(test_ip).await);
}

#[test]
fn test_origin_allowed_in_development() {
    // Development is an explicit opt-in (NODE_ENV=development): any origin works.
    // The process default is production so containers stay fail-closed.
    assert!(ws::is_origin_allowed(
        Some("https://evil.example"),
        "http://localhost:4402",
        "development"
    ));
    assert!(ws::is_origin_allowed(
        None,
        "http://localhost:4402",
        "development"
    ));
}

#[test]
fn test_origin_allowed_in_production_matches_base() {
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com"),
        "https://pad.example.com",
        "production"
    ));
    // Trailing slash on either side should not matter.
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com/"),
        "https://pad.example.com",
        "production"
    ));
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com"),
        "https://pad.example.com/",
        "production"
    ));
}

#[test]
fn test_origin_rejected_in_production() {
    // No Origin header in production is rejected (browsers always send one).
    assert!(!ws::is_origin_allowed(
        None,
        "https://pad.example.com",
        "production"
    ));

    // Mismatched origin is rejected.
    assert!(!ws::is_origin_allowed(
        Some("https://evil.example"),
        "https://pad.example.com",
        "production"
    ));
    assert!(!ws::is_origin_allowed(
        Some("http://pad.example.com"),
        "https://pad.example.com",
        "production"
    ));
}

#[test]
fn test_origin_does_not_honor_wildcard_base_in_production() {
    // The previous implementation had a `base_url == "*"` shortcut that
    // disabled the check. That shortcut is gone — a wildcard base in
    // production now rejects everything (forcing the operator to either
    // set the real base URL or run with NODE_ENV=development).
    assert!(!ws::is_origin_allowed(
        Some("https://anything"),
        "*",
        "production"
    ));
    assert!(!ws::is_origin_allowed(
        Some("https://anything"),
        "*",
        "production"
    ));
}

#[test]
fn test_sanitize_filename_rules() {
    use crate::migration::sanitize_filename;

    assert_eq!(sanitize_filename("hello").unwrap(), "hello");
    assert_eq!(sanitize_filename("my notepad").unwrap(), "my notepad");
    assert_eq!(sanitize_filename("a/b\\c").unwrap(), "a_b_c");

    // Path-traversal-class inputs are rejected.
    assert!(sanitize_filename("..").is_err());
    assert!(sanitize_filename(".").is_err());
    assert!(sanitize_filename("....").is_err());
    assert!(sanitize_filename(".hidden").is_err());

    // Windows-reserved names are rejected.
    assert!(sanitize_filename("CON").is_err());
    assert!(sanitize_filename("nul").is_err());

    // Empty / whitespace-only is rejected.
    assert!(sanitize_filename("").is_err());
    assert!(sanitize_filename("   ").is_err());
    assert!(sanitize_filename("///").is_err());
}

#[test]
fn test_property_sanitize_filename_never_panics() {
    use crate::migration::sanitize_filename;

    let long_str = "a".repeat(1000);
    let malformed_inputs = vec![
        "\0",
        "\x00\x01\x02",
        "../../../../etc/passwd",
        "C:\\Windows\\System32\\cmd.exe",
        "CON.txt",
        "AUX",
        "PRN.tar.gz",
        "COM1",
        "   ..   ",
        "🔥🦀🎉",
        long_str.as_str(),
        "../../..//..//",
        "hello\0world",
    ];

    for input in malformed_inputs {
        let res = sanitize_filename(input);
        if let Ok(clean) = res {
            assert!(!clean.contains('/'));
            assert!(!clean.contains('\\'));
            assert!(!clean.contains('\0'));
            assert!(!clean.starts_with('.'));
        }
    }
}

#[test]
fn test_property_origin_allowed_never_panics() {
    let origins = vec![
        None,
        Some(""),
        Some("http://localhost"),
        Some("https://pad.example.com"),
        Some("javascript:alert(1)"),
        Some("data:text/html,test"),
        Some("\0"),
    ];

    let bases = vec!["", "http://localhost:4402", "https://pad.example.com", "*"];
    let envs = vec!["production", "development", "staging", ""];

    for origin in &origins {
        for base in &bases {
            for env in &envs {
                let _ = ws::is_origin_allowed(*origin, base, env);
            }
        }
    }
}

#[test]
fn test_is_path_within_data_dir_security_helper() {
    use std::fs;

    let base_dir = std::env::temp_dir().join("pad_test_sec_helper");
    let _ = fs::create_dir_all(&base_dir);

    let valid_file = base_dir.join("notepad1.txt");
    fs::write(&valid_file, "content").unwrap();
    assert!(routes::notepads_crud::is_path_within_data_dir(
        &valid_file,
        &base_dir
    ));

    let escape_path = base_dir.join("../outside.txt");
    assert!(!routes::notepads_crud::is_path_within_data_dir(
        &escape_path,
        &base_dir
    ));

    let _ = fs::remove_file(&valid_file);
    let _ = fs::remove_dir(&base_dir);
}

#[tokio::test]
async fn test_generate_unique_name_collision_handling() {
    use std::collections::{HashMap, HashSet};
    use tokio::sync::RwLock;

    let config = AppConfig::load_from_env(4402);
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });

    let existing = vec![migration::Notepad {
        id: "1".to_string(),
        name: "Notes".to_string(),
    }];

    // Collision should append counter suffix
    let name = state.generate_unique_name("Notes", &existing);
    assert_eq!(name, "Notes-1");

    // Reserved "default" name should also be suffixed
    let default_name = state.generate_unique_name("default", &[]);
    assert_eq!(default_name, "default-1");
}

#[tokio::test]
async fn test_check_rate_limit_exceeds_budget_and_cleanup() {
    use std::collections::{HashMap, HashSet};
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::sync::RwLock;

    let config = AppConfig::load_from_env(4402);
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });
    let test_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

    // Fill up to 100 requests
    for _ in 0..100 {
        assert!(state.check_rate_limit(test_ip).await);
    }
    // 101st request should be rejected
    assert!(!state.check_rate_limit(test_ip).await);

    // Clean old rate limits
    state.clean_old_rate_limits().await;
}
