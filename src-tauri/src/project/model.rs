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

// --- Task (v0: type definitions only) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub task_id: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub input: serde_json::Value,
    pub segments: Vec<serde_json::Value>,
    pub output_assets: Vec<String>,
    pub error: Option<String>,
}

// --- Timeline ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub timeline_id: String,
    pub timebase: Timebase,
    pub tracks: Vec<Track>,
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
    pub clips: Vec<Clip>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub clip_id: String,
    pub asset_id: String,
    pub range: TimeRange,
    pub offset: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_context_asset_id: Option<String>,
    pub flags: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: f64,
    pub end: f64,
}

// --- Export ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRecord {
    pub export_id: String,
    pub status: String,
    pub preset: ExportPreset,
    pub range: TimeRange,
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
}

// --- Helper: rebuild indexes ---

impl ProjectFile {
    pub fn rebuild_indexes(&mut self) {
        self.indexes.asset_by_id.clear();
        self.indexes.task_by_id.clear();
        for (i, asset) in self.assets.iter().enumerate() {
            self.indexes
                .asset_by_id
                .insert(asset.asset_id.clone(), i);
        }
        for (i, task) in self.tasks.iter().enumerate() {
            self.indexes.task_by_id.insert(task.task_id.clone(), i);
        }
    }
}
