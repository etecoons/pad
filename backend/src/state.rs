use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::migration::{Notepad, migrate_default_notepad, sanitize_filename};
use crate::search::IndexedItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotepadsJson {
    pub notepads: Vec<Notepad>,
}

pub use crate::config::AppConfig;

pub struct AppStateInner {
    // Config
    pub config: AppConfig,
    pub data_dir: PathBuf,
    pub notepads_file: PathBuf,

    // Real-time clients map: userId -> UnboundedSender<Message>
    pub clients: RwLock<HashMap<String, UnboundedSender<Message>>>,

    // Operational Transformation (OT) history map: notepadId -> Operations
    pub operations_history: RwLock<HashMap<String, Vec<serde_json::Value>>>,

    // Active session IDs cache (random tokens, never the PIN).
    pub active_sessions: RwLock<std::collections::HashSet<String>>,

    // Per-IP request budget for general rate limiting (separate from PIN
    // brute-force lockouts, which now live in `shared_backend::auth::attempts`
    // and are global to the process).
    pub rate_limiter: RwLock<HashMap<IpAddr, Vec<Instant>>>,

    // Notepad metadata and index cache
    pub notepads: RwLock<Vec<Notepad>>,
    pub index_items: RwLock<Vec<IndexedItem>>,
    pub notepads_lock: tokio::sync::Mutex<()>,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub async fn ensure_data_dir(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.data_dir).await?;

        if fs::metadata(&self.notepads_file).await.is_err() {
            println!("Creating new notepads.json");
            let default_data = NotepadsJson {
                notepads: vec![Notepad {
                    id: "default".to_string(),
                    name: "default".to_string(),
                }],
            };
            let content = serde_json::to_string_pretty(&default_data)?;
            fs::write(&self.notepads_file, content).await?;
        } else {
            // Validate structure
            let content = fs::read_to_string(&self.notepads_file).await?;
            if let Err(e) = serde_json::from_str::<NotepadsJson>(&content) {
                eprintln!("Invalid notepads.json, recreating: {}", e);
                let default_data = NotepadsJson {
                    notepads: vec![Notepad {
                        id: "default".to_string(),
                        name: "default".to_string(),
                    }],
                };
                let content = serde_json::to_string_pretty(&default_data)?;
                fs::write(&self.notepads_file, content).await?;
            }
        }

        migrate_default_notepad(&self.data_dir).await?;
        Ok(())
    }

    pub async fn load_notepads_list(&self) -> Vec<Notepad> {
        self.get_notepads_from_dir().await.unwrap_or_default()
    }

    pub async fn get_notepads_from_dir(&self) -> Result<Vec<Notepad>, std::io::Error> {
        self.ensure_data_dir().await?;

        let file_content = fs::read_to_string(&self.notepads_file).await?;
        let mut data: NotepadsJson =
            serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });

        let mut read_dir = fs::read_dir(&self.data_dir).await?;
        let mut txt_files = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|ext| ext == "txt")
                && let Some(name_str) = path.file_stem().and_then(|s| s.to_str())
            {
                txt_files.push(name_str.to_string());
            }
        }

        // Find new files that don't match existing notepad IDs or sanitized names
        let mut new_notepads = Vec::new();
        for txt_file in txt_files {
            let matches_id = data.notepads.iter().any(|n| n.id == txt_file);
            let matches_sanitized_name = data
                .notepads
                .iter()
                .any(|n| sanitize_filename(&n.name).ok().as_deref() == Some(&txt_file));

            if !matches_id && !matches_sanitized_name {
                let unique_name = self.generate_unique_name(&txt_file, &data.notepads);
                new_notepads.push(Notepad {
                    id: txt_file,
                    name: unique_name,
                });
            }
        }

        if !new_notepads.is_empty() {
            data.notepads.extend(new_notepads.clone());
            let content = serde_json::to_string_pretty(&data)?;
            fs::write(&self.notepads_file, content).await?;
            println!(
                "Added new notepads: {}",
                data.notepads
                    .iter()
                    .map(|n| n.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        Ok(data.notepads)
    }

    pub fn generate_unique_name(&self, desired_name: &str, existing: &[Notepad]) -> String {
        let mut unique_name = desired_name.to_string();
        let mut counter = 1;

        while existing.iter().any(|n| n.name == unique_name)
            || sanitize_filename(&unique_name)
                .ok()
                .map(|s| s.to_lowercase() == "default")
                .unwrap_or(false)
        {
            unique_name = format!("{}-{}", desired_name, counter);
            counter += 1;
        }

        unique_name
    }

    pub async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let max_requests = 100; // 100 requests
        let window = Duration::from_secs(60); // per 60 seconds
        let now = Instant::now();

        let mut map = self.rate_limiter.write().await;
        let timestamps = map.entry(ip).or_insert_with(Vec::new);

        timestamps.retain(|&t| now.duration_since(t) < window);

        if timestamps.len() >= max_requests {
            false
        } else {
            timestamps.push(now);
            true
        }
    }

    pub async fn clean_old_rate_limits(&self) {
        let window = Duration::from_secs(60);
        let now = Instant::now();
        let mut map = self.rate_limiter.write().await;
        map.retain(|_, timestamps| {
            timestamps.retain(|&t| now.duration_since(t) < window);
            !timestamps.is_empty()
        });
    }
}
