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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_empty_project() -> ProjectFile {
        ProjectFile {
            schema_version: "0.2".to_string(),
            project: ProjectMeta {
                project_id: "proj_test".to_string(),
                name: "Test".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                settings: ProjectSettings {
                    fps: 24,
                    resolution: Resolution { width: 1920, height: 1080 },
                    aspect_ratio: "16:9".to_string(),
                    sample_rate: 48000,
                    generation: None,
                },
                paths: ProjectPaths {
                    workspace_root: "./workspace".to_string(),
                    assets_dir: "./workspace/assets".to_string(),
                    cache_dir: "./workspace/cache".to_string(),
                    exports_dir: "./workspace/exports".to_string(),
                },
                timeline_id: "tl_1".to_string(),
                default_draft_track_ids: DraftTrackIds {
                    video: "trk_v".to_string(),
                    audio: "trk_a".to_string(),
                    text: "trk_t".to_string(),
                },
            },
            assets: vec![],
            tasks: vec![],
            timeline: Timeline {
                timeline_id: "tl_1".to_string(),
                timebase: Timebase { fps: 24, unit: "seconds".to_string() },
                tracks: vec![
                    Track { track_id: "trk_v".to_string(), track_type: "video".to_string(), name: "Video".to_string(), clip_ids: vec![] },
                    Track { track_id: "trk_a".to_string(), track_type: "audio".to_string(), name: "Audio".to_string(), clip_ids: vec![] },
                    Track { track_id: "trk_t".to_string(), track_type: "text".to_string(), name: "Notes / Prompts".to_string(), clip_ids: vec![] },
                ],
                clips: HashMap::new(),
                markers: vec![],
                duration_ms: 0,
            },
            exports: vec![],
            indexes: Indexes {
                asset_by_id: HashMap::new(),
                task_by_id: HashMap::new(),
                clip_by_id: HashMap::new(),
            },
        }
    }

    fn make_prompt_asset(id: &str, label: &str) -> Asset {
        Asset {
            asset_id: id.to_string(),
            asset_type: "prompt".to_string(),
            source: "authored".to_string(),
            fingerprint: Fingerprint {
                algo: "sha256".to_string(),
                value: format!("sha256:{}", id),
                basis: "content_json".to_string(),
            },
            path: format!("workspace/assets/prompts/{}.md", id),
            meta: serde_json::json!({
                "kind": "prompt",
                "language": "zh",
                "format": "markdown",
                "label": label,
            }),
            generation: None,
            tags: vec!["prompt".to_string()],
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn prompt_asset_added_and_indexed() {
        let mut pf = make_empty_project();
        let asset = make_prompt_asset("ast_prompt_001", "test note");
        pf.assets.push(asset);
        pf.rebuild_indexes();

        assert_eq!(pf.assets.len(), 1);
        assert_eq!(pf.indexes.asset_by_id.get("ast_prompt_001"), Some(&0));
        assert_eq!(pf.assets[0].asset_type, "prompt");
    }

    #[test]
    fn prompt_asset_meta_fields() {
        let asset = make_prompt_asset("ast_prompt_002", "my label");
        assert_eq!(asset.meta["kind"], "prompt");
        assert_eq!(asset.meta["language"], "zh");
        assert_eq!(asset.meta["format"], "markdown");
        assert_eq!(asset.meta["label"], "my label");
    }

    #[test]
    fn prompt_clip_on_text_track() {
        let mut pf = make_empty_project();
        let asset = make_prompt_asset("ast_prompt_003", "clip test");
        pf.assets.push(asset);

        let clip = Clip {
            clip_id: "clip_001".to_string(),
            asset_id: "ast_prompt_003".to_string(),
            track_id: "trk_t".to_string(),
            start_ms: 1000,
            duration_ms: 5000,
            in_ms: 0,
            out_ms: 5000,
        };

        let text_track = pf.timeline.tracks.iter_mut()
            .find(|t| t.track_type == "text").unwrap();
        text_track.clip_ids.push("clip_001".to_string());
        pf.timeline.clips.insert("clip_001".to_string(), clip);
        pf.timeline.recalc_duration();
        pf.rebuild_indexes();

        assert_eq!(pf.timeline.duration_ms, 6000);
        assert_eq!(pf.indexes.clip_by_id.get("clip_001"), Some(&"trk_t".to_string()));
        let text_track = pf.timeline.tracks.iter()
            .find(|t| t.track_type == "text").unwrap();
        assert_eq!(text_track.clip_ids, vec!["clip_001"]);
    }

    #[test]
    fn prompt_clip_at_playhead_position() {
        let mut pf = make_empty_project();
        let asset = make_prompt_asset("ast_prompt_004", "");
        pf.assets.push(asset);

        let playhead_ms = 3500;
        let clip = Clip {
            clip_id: "clip_ph".to_string(),
            asset_id: "ast_prompt_004".to_string(),
            track_id: "trk_t".to_string(),
            start_ms: playhead_ms,
            duration_ms: 5000,
            in_ms: 0,
            out_ms: 5000,
        };

        pf.timeline.clips.insert("clip_ph".to_string(), clip.clone());
        pf.timeline.tracks.iter_mut()
            .find(|t| t.track_type == "text").unwrap()
            .clip_ids.push("clip_ph".to_string());
        pf.timeline.recalc_duration();

        assert_eq!(clip.start_ms, 3500);
        assert_eq!(pf.timeline.duration_ms, 8500);
    }

    #[test]
    fn multiple_prompt_assets_indexed_correctly() {
        let mut pf = make_empty_project();
        pf.assets.push(make_prompt_asset("p1", "first"));
        pf.assets.push(make_prompt_asset("p2", "second"));
        pf.assets.push(make_prompt_asset("p3", "third"));
        pf.rebuild_indexes();

        assert_eq!(pf.indexes.asset_by_id.len(), 3);
        assert_eq!(pf.indexes.asset_by_id["p1"], 0);
        assert_eq!(pf.indexes.asset_by_id["p2"], 1);
        assert_eq!(pf.indexes.asset_by_id["p3"], 2);
    }

    #[test]
    fn prompt_asset_serializes_correctly() {
        let asset = make_prompt_asset("ast_prompt_ser", "ser test");
        let json = serde_json::to_value(&asset).unwrap();

        assert_eq!(json["type"], "prompt");
        assert_eq!(json["source"], "authored");
        assert_eq!(json["assetId"], "ast_prompt_ser");
        assert_eq!(json["meta"]["kind"], "prompt");
        assert_eq!(json["tags"], serde_json::json!(["prompt"]));
    }

    #[test]
    fn recalc_duration_with_no_clips() {
        let mut pf = make_empty_project();
        pf.timeline.recalc_duration();
        assert_eq!(pf.timeline.duration_ms, 0);
    }

    #[test]
    fn rebuild_indexes_clears_stale_entries() {
        let mut pf = make_empty_project();
        pf.assets.push(make_prompt_asset("p1", "a"));
        pf.rebuild_indexes();
        assert_eq!(pf.indexes.asset_by_id.len(), 1);

        pf.assets.clear();
        pf.rebuild_indexes();
        assert_eq!(pf.indexes.asset_by_id.len(), 0);
    }
}
