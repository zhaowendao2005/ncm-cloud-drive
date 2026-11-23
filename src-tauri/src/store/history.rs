use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::state::{load_json, save_json, MAX_HISTORY};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryItem {
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    #[serde(rename = "fileMd5")]
    pub file_md5: String,
    #[serde(rename = "fileId")]
    pub file_id: i64,
    #[serde(rename = "taskId")]
    pub task_id: i64,
    pub fast: bool,
    pub status: String,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: i64,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    #[serde(rename = "avgSpeed")]
    pub avg_speed: f64,
}

fn path(dir: &Path) -> std::path::PathBuf {
    dir.join("history.json")
}

pub fn list(dir: &Path) -> Vec<HistoryItem> {
    load_json::<Vec<HistoryItem>>(&path(dir)).unwrap_or_default()
}

pub fn append(dir: &Path, item: HistoryItem) {
    let mut items = list(dir);
    items.retain(|i| !(i.file_md5 == item.file_md5 && i.file_size == item.file_size));
    items.insert(0, item);
    items.truncate(MAX_HISTORY);
    let _ = save_json(&path(dir), &items);
}

pub fn clear(dir: &Path) {
    let _ = std::fs::remove_file(path(dir));
}
