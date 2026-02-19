use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// Cutline Project JSON v0 — Rust 数据结构
// project.json 只存元数据和相对路径引用
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub schema_version: String,
    pub project: ProjectMeta,
    pub assets: Vec<Asset>,
    pub tasks: Vec<Task>,
    pub timeline: Timeline,
    pub exports: Vec<ExportRecord>,
    pub indexes: Indexes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMeta {
    pub project_id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub settings: ProjectSettings,
    pub paths: ProjectPaths,
    pub timeline_id: String,
    pub default_draft_track_ids: DraftTrackIds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    pub fps: u32,
    pub resolution: Resolution,
    pub aspect_ratio: String,
    pub sample_rate: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<GenerationSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPaths {
    pub workspace_root: String,
    pub assets_dir: String,
    pub cache_dir: String,
    pub exports_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftTrackIds {
    pub video: String,
    pub audio: String,
    pub text: String,
}

// --- Asset ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub asset_id: String,
    #[serde(rename = "type")]
    pub asset_type: String,
    pub source: String,
    pub fingerprint: Fingerprint,
    pub path: String,
    pub meta: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<GenerationInfo>,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    pub algo: String,
    pub value: String,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationInfo {
    #[serde(rename = "taskId")]
    pub task_id: String,
    pub model: String,
    pub params: serde_json::Value,
}

// --- Task v1 ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub task_id: String,
    pub kind: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<TaskProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TaskError>,
    pub retries: TaskRetries,
    pub deps: Vec<String>,
    pub events: Vec<TaskEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,
}

pub const MAX_TASK_EVENTS: usize = 200;

impl Task {
    pub fn append_event(&mut self, level: &str, msg: &str) {
        self.events.push(TaskEvent {
            t: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            msg: msg.to_string(),
        });
        if self.events.len() > MAX_TASK_EVENTS {
            let drain_count = self.events.len() - MAX_TASK_EVENTS;
            self.events.drain(0..drain_count);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRetries {
    pub count: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub t: String,
    pub level: String,
    pub msg: String,
}

// --- Timeline v2 (normalized, ms integers) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub timeline_id: String,
    pub timebase: Timebase,
    pub tracks: Vec<Track>,
    pub clips: HashMap<String, Clip>,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub duration_ms: i64,
}

impl Timeline {
    pub fn recalc_duration(&mut self) {
        self.duration_ms = self
            .clips
            .values()
            .map(|c| c.start_ms + c.duration_ms)
            .max()
            .unwrap_or(0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timebase {
    pub fps: u32,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub track_id: String,
    #[serde(rename = "type")]
    pub track_type: String,
    pub name: String,
    pub clip_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub clip_id: String,
    pub asset_id: String,
    pub track_id: String,
    pub start_ms: i64,
    pub duration_ms: i64,
    pub in_ms: i64,
    pub out_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub marker_id: String,
    pub t_ms: i64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub prompt_text: String,
    pub created_at: String,
}

// --- Export ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRecord {
    pub export_id: String,
    pub status: String,
    pub preset: ExportPreset,
    pub start_ms: i64,
    pub end_ms: i64,
    pub output_uri: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreset {
    pub container: String,
    pub codec: String,
    pub bitrate_kbps: u32,
}

// --- Indexes ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Indexes {
    pub asset_by_id: HashMap<String, usize>,
    pub task_by_id: HashMap<String, usize>,
    #[serde(default)]
    pub clip_by_id: HashMap<String, String>,
}

// --- Helper: rebuild indexes ---

impl ProjectFile {
    pub fn rebuild_indexes(&mut self) {
        self.indexes.asset_by_id.clear();
        self.indexes.task_by_id.clear();
        self.indexes.clip_by_id.clear();
        for (i, asset) in self.assets.iter().enumerate() {
            self.indexes
                .asset_by_id
                .insert(asset.asset_id.clone(), i);
        }
        for (i, task) in self.tasks.iter().enumerate() {
            self.indexes.task_by_id.insert(task.task_id.clone(), i);
        }
        for (clip_id, clip) in &self.timeline.clips {
            self.indexes
                .clip_by_id
                .insert(clip_id.clone(), clip.track_id.clone());
        }
    }
}
