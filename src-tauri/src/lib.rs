mod asset;
mod media;
mod project;
mod provider;
mod providers;
mod secrets;
mod state;
mod task;

use project::model::{
    Asset, Clip, DraftTrackIds, Indexes, Marker, ProjectFile, ProjectMeta, ProjectPaths,
    ProjectSettings, Resolution, Task, TaskError, TaskEvent, TaskRetries, Timeline, Timebase, Track,
};
use state::{AppState, LoadedProject};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Emitter;

// ============================================================
// Tauri Commands
// ============================================================

#[tauri::command]
async fn create_project(
    dir_path: String,
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<ProjectFile, String> {
    let project_dir = PathBuf::from(&dir_path);
    if !project_dir.exists() {
        std::fs::create_dir_all(&project_dir)
            .map_err(|e| format!("创建项目目录失败: {}", e))?;
    }

    project::io::ensure_workspace_dirs(&project_dir)?;

    let timeline_id = format!("tl_{}", uuid::Uuid::new_v4());
    let video_track_id = format!("trk_v_{}", uuid::Uuid::new_v4());
    let audio_track_id = format!("trk_a_{}", uuid::Uuid::new_v4());
    let text_track_id = format!("trk_t_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();

    let pf = ProjectFile {
        schema_version: "0.2".to_string(),
        project: ProjectMeta {
            project_id: format!("proj_{}", uuid::Uuid::new_v4()),
            name,
            created_at: now.clone(),
            updated_at: now,
            settings: ProjectSettings {
                fps: 24,
                resolution: Resolution {
                    width: 1920,
                    height: 1080,
                },
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
            timeline_id: timeline_id.clone(),
            default_draft_track_ids: DraftTrackIds {
                video: video_track_id.clone(),
                audio: audio_track_id.clone(),
                text: text_track_id.clone(),
            },
        },
        assets: vec![],
        tasks: vec![],
        timeline: Timeline {
            timeline_id,
            timebase: Timebase {
                fps: 24,
                unit: "seconds".to_string(),
            },
            tracks: vec![
                Track {
                    track_id: video_track_id,
                    track_type: "video".to_string(),
                    name: "Draft Video".to_string(),
                    clip_ids: vec![],
                },
                Track {
                    track_id: audio_track_id,
                    track_type: "audio".to_string(),
                    name: "Draft Audio".to_string(),
                    clip_ids: vec![],
                },
                Track {
                    track_id: text_track_id,
                    track_type: "text".to_string(),
                    name: "Notes / Prompts".to_string(),
                    clip_ids: vec![],
                },
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
    };

    let project_json_path = project_dir.join("project.json");
    project::io::write_project_atomic(&project_json_path, &pf)?;

    // Load into AppState
    let mut guard = state.inner.lock().await;
    *guard = Some(LoadedProject {
        project: pf.clone(),
        json_path: project_json_path,
        project_dir,
        dirty: false,
    });

    Ok(pf)
}

#[tauri::command]
async fn open_project(
    project_json_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<ProjectFile, String> {
    let path = PathBuf::from(&project_json_path);
    let mut pf = project::io::read_project(&path)?;

    // Crash recovery: mark running tasks as failed
    let now = chrono::Utc::now().to_rfc3339();
    for task in &mut pf.tasks {
        if task.state == "running" {
            task.state = "failed".to_string();
            task.updated_at = now.clone();
            task.error = Some(TaskError {
                code: "crash_recovered".to_string(),
                message: "Task was running when app exited.".to_string(),
                detail: None,
            });
            task.events.push(TaskEvent {
                t: now.clone(),
                level: "warn".to_string(),
                msg: "crash_recovered: task was running when app exited".to_string(),
            });
        }
    }

    let project_dir = path
        .parent()
        .ok_or("无法获取项目目录")?
        .to_path_buf();

    // Ensure cache dirs exist
    project::io::ensure_workspace_dirs(&project_dir)?;

    // Save crash recovery changes
    pf.rebuild_indexes();
    project::io::write_project_atomic(&path, &pf)?;

    // Load into AppState
    let mut guard = state.inner.lock().await;
    *guard = Some(LoadedProject {
        project: pf.clone(),
        json_path: path,
        project_dir,
        dirty: false,
    });

    Ok(pf)
}

#[tauri::command]
async fn save_project(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;
    loaded.project.rebuild_indexes();
    loaded.project.project.updated_at = chrono::Utc::now().to_rfc3339();
    project::io::write_project_atomic(&loaded.json_path, &loaded.project)?;
    loaded.dirty = false;
    Ok(())
}

#[tauri::command]
async fn get_project(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<ProjectFile, String> {
    let guard = state.inner.lock().await;
    let loaded = guard.as_ref().ok_or("没有打开的项目")?;
    Ok(loaded.project.clone())
}

#[tauri::command]
async fn import_assets(
    file_paths: Vec<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<Asset>, String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let mut new_assets: Vec<Asset> = Vec::new();
    let mut thumb_tasks: Vec<(String, String)> = Vec::new(); // (taskId, assetId)

    for file_path_str in &file_paths {
        let source_path = PathBuf::from(file_path_str);
        if !source_path.exists() {
            return Err(format!("文件不存在: {}", file_path_str));
        }

        let fp = asset::fingerprint::compute_file_fingerprint(&source_path)?;

        if asset::registry::find_duplicate(&loaded.project.assets, &fp.value).is_some() {
            continue;
        }

        let asset_type = guess_asset_type(&source_path);
        let sub_dir = match asset_type.as_str() {
            "video" => "workspace/assets/video",
            "audio" => "workspace/assets/audio",
            "image" => "workspace/assets/images",
            _ => "workspace/assets/video",
        };

        let file_name = source_path
            .file_name()
            .ok_or("无法获取文件名")?
            .to_string_lossy()
            .to_string();

        let dest_dir = loaded.project_dir.join(sub_dir);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("创建目录失败: {}", e))?;

        let dest_path = dest_dir.join(&file_name);

        if !dest_path.exists() {
            std::fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("复制文件失败: {}", e))?;
        }

        let relative_path = format!("{}/{}", sub_dir, file_name);

        let meta = match asset_type.as_str() {
            "video" | "audio" => match media::probe::ffprobe(&dest_path) {
                Ok(probe_data) => media::probe::extract_video_meta(&probe_data),
                Err(_) => serde_json::json!({ "kind": asset_type }),
            },
            "image" => media::probe::extract_image_meta(&dest_path),
            _ => serde_json::json!({ "kind": "unknown" }),
        };

        let asset_id = format!(
            "ast_{}_{}",
            asset_type,
            &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
        );

        let asset = Asset {
            asset_id: asset_id.clone(),
            asset_type: asset_type.clone(),
            source: "uploaded".to_string(),
            fingerprint: fp,
            path: relative_path,
            meta,
            generation: None,
            tags: vec!["source".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        loaded.project.assets.push(asset.clone());
        new_assets.push(asset);

        // Auto-enqueue thumb task for video/image
        if asset_type == "video" || asset_type == "image" {
            let now = chrono::Utc::now().to_rfc3339();
            let thumb_task_id = format!("task_thumb_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]);
            let thumb_task = Task {
                task_id: thumb_task_id.clone(),
                kind: "thumb".to_string(),
                state: "queued".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
                input: serde_json::json!({ "assetId": asset_id }),
                output: None,
                progress: None,
                error: None,
                retries: TaskRetries { count: 0, max: 3 },
                deps: vec![],
                events: vec![TaskEvent {
                    t: now.clone(),
                    level: "info".to_string(),
                    msg: "Task enqueued (auto: import)".to_string(),
                }],
                dedupe_key: Some(format!("thumb:{}", asset_id)),
            };
            loaded.project.tasks.push(thumb_task);
            thumb_tasks.push((thumb_task_id.clone(), asset_id.clone()));

            // Auto-enqueue proxy task for video (depends on thumb)
            if asset_type == "video" {
                let proxy_task_id = format!("task_proxy_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]);
                let proxy_task = Task {
                    task_id: proxy_task_id,
                    kind: "proxy".to_string(),
                    state: "queued".to_string(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    input: serde_json::json!({ "assetId": asset_id }),
                    output: None,
                    progress: None,
                    error: None,
                    retries: TaskRetries { count: 0, max: 3 },
                    deps: vec![thumb_task_id],
                    events: vec![TaskEvent {
                        t: now,
                        level: "info".to_string(),
                        msg: "Task enqueued (auto: import)".to_string(),
                    }],
                    dedupe_key: Some(format!("proxy:{}", asset_id)),
                };
                loaded.project.tasks.push(proxy_task);
            }
        }
    }

    loaded.project.rebuild_indexes();
    loaded.project.project.updated_at = chrono::Utc::now().to_rfc3339();
    loaded.dirty = true;

    // Save immediately after import
    project::io::write_project_atomic(&loaded.json_path, &loaded.project)?;
    loaded.dirty = false;

    // Notify task runner
    drop(guard);
    state.task_notify.notify_one();

    Ok(new_assets)
}

#[tauri::command]
fn probe_media(file_path: String) -> Result<serde_json::Value, String> {
    let path = Path::new(&file_path);
    let probe_data = media::probe::ffprobe(path)?;
    Ok(media::probe::extract_video_meta(&probe_data))
}

// ============================================================
// File Access
// ============================================================

#[tauri::command]
async fn read_file_base64(
    relative_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let guard = state.inner.lock().await;
    let loaded = guard.as_ref().ok_or("没有打开的项目")?;
    let abs_path = loaded.project_dir.join(&relative_path);
    drop(guard);

    let bytes = std::fs::read(&abs_path)
        .map_err(|e| format!("读取文件失败 {}: {}", abs_path.display(), e))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

// ============================================================
// Task Commands
// ============================================================

#[tauri::command]
async fn task_enqueue(
    kind: String,
    input: serde_json::Value,
    deps: Option<Vec<String>>,
    dedupe_key: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    // Check deduplication
    if let Some(ref dk) = dedupe_key {
        let existing = loaded.project.tasks.iter().find(|t| {
            t.dedupe_key.as_deref() == Some(dk) && t.state == "succeeded"
        });
        if existing.is_some() {
            return Err(format!("已存在成功的同类任务 (dedupeKey: {})", dk));
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let task_id = format!(
        "task_{}_{}",
        kind,
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );

    let task = Task {
        task_id: task_id.clone(),
        kind,
        state: "queued".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        input,
        output: None,
        progress: None,
        error: None,
        retries: TaskRetries { count: 0, max: 3 },
        deps: deps.unwrap_or_default(),
        events: vec![TaskEvent {
            t: now,
            level: "info".to_string(),
            msg: "Task enqueued".to_string(),
        }],
        dedupe_key,
    };

    loaded.project.tasks.push(task);
    loaded.project.rebuild_indexes();
    loaded.dirty = true;

    drop(guard);
    state.save_notify.notify_one();
    state.task_notify.notify_one();

    Ok(task_id)
}

#[tauri::command]
async fn task_retry(
    task_id: String,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let task = loaded
        .project
        .tasks
        .iter_mut()
        .find(|t| t.task_id == task_id)
        .ok_or(format!("任务不存在: {}", task_id))?;

    if task.state != "failed" && task.state != "canceled" {
        return Err(format!("只能重试 failed/canceled 状态的任务，当前: {}", task.state));
    }

    task.state = "queued".to_string();
    task.updated_at = chrono::Utc::now().to_rfc3339();
    task.retries.count += 1;
    task.error = None;
    task.progress = None;
    task.append_event("info", &format!("Task retried (attempt #{})", task.retries.count));

    let snapshot = task.clone();
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("task:updated", serde_json::json!({ "task": snapshot }));
    state.save_notify.notify_one();
    state.task_notify.notify_one();

    Ok(())
}

#[tauri::command]
async fn task_cancel(
    task_id: String,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let task = loaded
        .project
        .tasks
        .iter_mut()
        .find(|t| t.task_id == task_id)
        .ok_or(format!("任务不存在: {}", task_id))?;

    match task.state.as_str() {
        "queued" => {
            task.state = "canceled".to_string();
            task.updated_at = chrono::Utc::now().to_rfc3339();
            task.append_event("warn", "Task canceled (was queued)");
            let snapshot = task.clone();
            loaded.dirty = true;
            drop(guard);
            let _ = app_handle.emit("task:updated", serde_json::json!({ "task": snapshot }));
            state.save_notify.notify_one();
        }
        "running" => {
            // Set cancel flag; runner will check it
            drop(guard);
            let mut flags = state.cancel_flags.lock().await;
            flags.insert(task_id);
        }
        _ => {
            return Err(format!("无法取消状态为 {} 的任务", task.state));
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskSummary {
    task_id: String,
    kind: String,
    state: String,
    created_at: String,
    updated_at: String,
    progress: Option<project::model::TaskProgress>,
    error: Option<project::model::TaskError>,
    retries: project::model::TaskRetries,
}

#[tauri::command]
async fn task_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<TaskSummary>, String> {
    let guard = state.inner.lock().await;
    let loaded = guard.as_ref().ok_or("没有打开的项目")?;

    let summaries: Vec<TaskSummary> = loaded
        .project
        .tasks
        .iter()
        .map(|t| TaskSummary {
            task_id: t.task_id.clone(),
            kind: t.kind.clone(),
            state: t.state.clone(),
            created_at: t.created_at.clone(),
            updated_at: t.updated_at.clone(),
            progress: t.progress.clone(),
            error: t.error.clone(),
            retries: t.retries.clone(),
        })
        .collect();

    Ok(summaries)
}

// ============================================================
// Timeline Commands
// ============================================================

#[tauri::command]
async fn timeline_add_clip(
    track_id: String,
    asset_id: String,
    start_ms: i64,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<Clip, String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let asset = loaded
        .project
        .assets
        .iter()
        .find(|a| a.asset_id == asset_id)
        .ok_or(format!("Asset not found: {}", asset_id))?;

    let duration_sec = asset
        .meta
        .get("durationSec")
        .and_then(|v| v.as_f64())
        .unwrap_or(5.0);
    let duration_ms = (duration_sec * 1000.0) as i64;

    let track = loaded
        .project
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.track_id == track_id)
        .ok_or(format!("Track not found: {}", track_id))?;

    let clip_id = format!(
        "clip_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );

    let clip = Clip {
        clip_id: clip_id.clone(),
        asset_id,
        track_id: track_id.clone(),
        start_ms: start_ms.max(0),
        duration_ms,
        in_ms: 0,
        out_ms: duration_ms,
    };

    track.clip_ids.push(clip_id.clone());
    loaded
        .project
        .timeline
        .clips
        .insert(clip_id.clone(), clip.clone());
    loaded.project.timeline.recalc_duration();
    loaded.project.rebuild_indexes();
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(clip)
}

#[tauri::command]
async fn timeline_move_clip(
    clip_id: String,
    new_start_ms: i64,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let clip = loaded
        .project
        .timeline
        .clips
        .get_mut(&clip_id)
        .ok_or(format!("Clip not found: {}", clip_id))?;

    clip.start_ms = new_start_ms.max(0);
    loaded.project.timeline.recalc_duration();
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(())
}

#[tauri::command]
async fn timeline_trim_clip(
    clip_id: String,
    in_ms: Option<i64>,
    out_ms: Option<i64>,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let clip = loaded
        .project
        .timeline
        .clips
        .get_mut(&clip_id)
        .ok_or(format!("Clip not found: {}", clip_id))?;

    if let Some(new_in) = in_ms {
        if new_in < 0 {
            return Err("inMs cannot be negative".to_string());
        }
        clip.in_ms = new_in;
    }
    if let Some(new_out) = out_ms {
        clip.out_ms = new_out;
    }

    if clip.out_ms <= clip.in_ms {
        return Err("outMs must be greater than inMs".to_string());
    }

    clip.duration_ms = clip.out_ms - clip.in_ms;
    loaded.project.timeline.recalc_duration();
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(())
}

#[tauri::command]
async fn timeline_remove_clip(
    clip_id: String,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    loaded.project.timeline.clips.remove(&clip_id);

    for track in &mut loaded.project.timeline.tracks {
        track.clip_ids.retain(|id| id != &clip_id);
    }

    loaded.project.timeline.recalc_duration();
    loaded.project.rebuild_indexes();
    loaded.dirty = true;

    // Force save on deletion
    project::io::write_project_atomic(&loaded.json_path, &loaded.project)?;
    loaded.dirty = false;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());

    Ok(())
}

#[tauri::command]
async fn timeline_reorder_clips(
    track_id: String,
    clip_ids: Vec<String>,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let track = loaded
        .project
        .timeline
        .tracks
        .iter_mut()
        .find(|t| t.track_id == track_id)
        .ok_or(format!("Track not found: {}", track_id))?;

    for cid in &clip_ids {
        if !track.clip_ids.contains(cid) {
            return Err(format!("Clip {} not in track {}", cid, track_id));
        }
    }

    track.clip_ids = clip_ids;
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(())
}

// ============================================================
// Marker Commands
// ============================================================

#[tauri::command]
async fn marker_add(
    t_ms: i64,
    label: Option<String>,
    prompt_text: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<Marker, String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let marker = Marker {
        marker_id: format!(
            "mkr_{}",
            &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
        ),
        t_ms,
        label: label.unwrap_or_default(),
        prompt_text: prompt_text.unwrap_or_default(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    loaded.project.timeline.markers.push(marker.clone());
    loaded
        .project
        .timeline
        .markers
        .sort_by_key(|m| m.t_ms);
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(marker)
}

#[tauri::command]
async fn marker_update(
    marker_id: String,
    label: Option<String>,
    prompt_text: Option<String>,
    t_ms: Option<i64>,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let marker = loaded
        .project
        .timeline
        .markers
        .iter_mut()
        .find(|m| m.marker_id == marker_id)
        .ok_or(format!("Marker not found: {}", marker_id))?;

    if let Some(l) = label {
        marker.label = l;
    }
    if let Some(p) = prompt_text {
        marker.prompt_text = p;
    }
    if let Some(t) = t_ms {
        marker.t_ms = t;
    }

    loaded
        .project
        .timeline
        .markers
        .sort_by_key(|m| m.t_ms);
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(())
}

#[tauri::command]
async fn marker_remove(
    marker_id: String,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    let before_len = loaded.project.timeline.markers.len();
    loaded
        .project
        .timeline
        .markers
        .retain(|m| m.marker_id != marker_id);

    if loaded.project.timeline.markers.len() == before_len {
        return Err(format!("Marker not found: {}", marker_id));
    }

    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(())
}

// ============================================================
// Project Settings Commands
// ============================================================

#[tauri::command]
async fn update_generation_settings(
    video_provider: Option<String>,
    video_profile: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let loaded = guard.as_mut().ok_or("没有打开的项目")?;

    loaded.project.project.settings.generation = Some(
        project::model::GenerationSettings {
            video_provider,
            video_profile,
        },
    );
    loaded.project.project.updated_at = chrono::Utc::now().to_rfc3339();
    loaded.dirty = true;

    drop(guard);
    let _ = app_handle.emit("project:updated", ());
    state.save_notify.notify_one();

    Ok(())
}

// ============================================================
// Provider Commands
// ============================================================

#[tauri::command]
async fn providers_list(
    app_handle: tauri::AppHandle,
) -> Result<Vec<provider::model::ProviderSummary>, String> {
    let path = provider::io::providers_path(&app_handle)?;
    let file = provider::io::load_providers(&path)?;
    let mut list: Vec<provider::model::ProviderSummary> = file
        .providers
        .iter()
        .map(|(name, cfg)| provider::model::ProviderSummary {
            name: name.clone(),
            display_name: cfg.display_name.clone(),
            auth_kind: cfg.auth.kind.clone(),
            profiles: cfg.profiles.keys().cloned().collect(),
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

#[tauri::command]
async fn providers_get(
    name: String,
    app_handle: tauri::AppHandle,
) -> Result<provider::model::ProviderConfig, String> {
    let path = provider::io::providers_path(&app_handle)?;
    let file = provider::io::load_providers(&path)?;
    file.providers
        .get(&name)
        .cloned()
        .ok_or(format!("provider_not_found: {}", name))
}

#[tauri::command]
async fn providers_upsert(
    name: String,
    config: provider::model::ProviderConfig,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let path = provider::io::providers_path(&app_handle)?;
    let mut file = provider::io::load_providers(&path)?;
    file.providers.insert(name, config);
    provider::io::save_providers_atomic(&path, &file)
}

#[tauri::command]
async fn providers_delete(
    name: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let path = provider::io::providers_path(&app_handle)?;
    let mut file = provider::io::load_providers(&path)?;
    file.providers.remove(&name);
    provider::io::save_providers_atomic(&path, &file)
}

#[tauri::command]
async fn secrets_set(
    credential_ref: String,
    secret: String,
) -> Result<(), String> {
    secrets::set_secret(&credential_ref, &secret)
}

#[tauri::command]
async fn secrets_exists(
    credential_ref: String,
) -> Result<bool, String> {
    secrets::exists(&credential_ref)
}

#[tauri::command]
async fn secrets_delete(
    credential_ref: String,
) -> Result<(), String> {
    secrets::delete_secret(&credential_ref)
}

#[tauri::command]
async fn providers_test(
    provider_name: String,
    profile_name: String,
    app_handle: tauri::AppHandle,
) -> Result<provider::model::TestResult, String> {
    Ok(provider::test::run_provider_test(&app_handle, &provider_name, &profile_name).await)
}

// ============================================================
// Jimeng Provider Commands
// ============================================================

/// Helper: build a JimengClient from provider config + keyring, or a direct token.
async fn build_jimeng_client(
    app_handle: &tauri::AppHandle,
    provider_name: &str,
    profile_name: &str,
    token_override: Option<&str>,
) -> Result<providers::jimeng::client::JimengClient, String> {
    let path = provider::io::providers_path(app_handle)?;
    let file = provider::io::load_providers(&path)?;
    let prov = file
        .providers
        .get(provider_name)
        .ok_or(format!("provider_not_found: {}", provider_name))?;
    let profile = prov
        .profiles
        .get(profile_name)
        .ok_or(format!("profile_not_found: {}", profile_name))?;

    let secret = match token_override {
        Some(t) => t.to_string(),
        None => secrets::get_secret(&profile.credential_ref)?
            .ok_or("missing_credentials")?,
    };

    let timeout_secs = profile.timeout_ms / 1000;
    providers::jimeng::client::JimengClient::new(
        &secret,
        Some(prov.base_url.as_str()),
        timeout_secs.max(10),
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn jimeng_generate_image(
    provider_name: String,
    profile_name: String,
    prompt: String,
    model: Option<String>,
    ratio: Option<String>,
    negative_prompt: Option<String>,
    image_count: Option<u32>,
    token: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<providers::jimeng::api::GenerateResult, String> {
    let client = build_jimeng_client(&app_handle, &provider_name, &profile_name, token.as_deref()).await?;
    providers::jimeng::api::generate_image(
        &client,
        &prompt,
        model.as_deref().unwrap_or("jimeng-4.5"),
        ratio.as_deref().unwrap_or("1:1"),
        negative_prompt.as_deref().unwrap_or(""),
        image_count.unwrap_or(4),
    )
    .await
}

#[tauri::command]
async fn jimeng_task_status(
    provider_name: String,
    profile_name: String,
    history_ids: Vec<String>,
    token: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<HashMap<String, providers::jimeng::api::TaskStatusResult>, String> {
    let client = build_jimeng_client(&app_handle, &provider_name, &profile_name, token.as_deref()).await?;
    providers::jimeng::api::get_task_status(&client, &history_ids, None).await
}

#[tauri::command]
async fn jimeng_credit_balance(
    provider_name: String,
    profile_name: String,
    token: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<providers::jimeng::api::CreditInfo, String> {
    let client = build_jimeng_client(&app_handle, &provider_name, &profile_name, token.as_deref()).await?;
    providers::jimeng::api::get_credit(&client).await
}

// ============================================================
// gen_video / export commands
// ============================================================

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn gen_video_enqueue(
    provider_name: String,
    profile_name: String,
    prompt: String,
    model: Option<String>,
    ratio: Option<String>,
    duration_ms: Option<u32>,
    start_ms: Option<i64>,
    token: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let task_id = format!(
        "task_gen_video_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );

    let mut input = serde_json::json!({
        "providerName": provider_name,
        "profileName": profile_name,
        "prompt": prompt,
    });
    if let Some(m) = &model {
        input["model"] = serde_json::json!(m);
    }
    if let Some(r) = &ratio {
        input["ratio"] = serde_json::json!(r);
    }
    if let Some(d) = duration_ms {
        input["durationMs"] = serde_json::json!(d);
    }
    if let Some(s) = start_ms {
        input["startMs"] = serde_json::json!(s);
    }
    if let Some(t) = &token {
        input["token"] = serde_json::json!(t);
    }

    let task = Task {
        task_id: task_id.clone(),
        kind: "gen_video".to_string(),
        state: "queued".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        input,
        output: None,
        progress: None,
        error: None,
        retries: TaskRetries { count: 0, max: 2 },
        deps: vec![],
        events: vec![TaskEvent {
            t: now,
            level: "info".to_string(),
            msg: "gen_video task enqueued".to_string(),
        }],
        dedupe_key: None,
    };

    {
        let mut guard = state.inner.lock().await;
        let loaded = guard.as_mut().ok_or("No project loaded")?;
        loaded.project.tasks.push(task.clone());
        loaded.project.rebuild_indexes();
        loaded.dirty = true;
    }

    state.task_notify.notify_one();
    let _ = app_handle.emit("task:updated", serde_json::json!({ "task": task }));

    Ok(serde_json::json!({ "taskId": task_id }))
}

#[tauri::command]
async fn export_draft(
    track_id: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let task_id = format!(
        "task_export_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );

    let input = serde_json::json!({
        "trackId": track_id.unwrap_or_else(|| "trk_draft".to_string()),
    });

    let task = Task {
        task_id: task_id.clone(),
        kind: "export".to_string(),
        state: "queued".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        input,
        output: None,
        progress: None,
        error: None,
        retries: TaskRetries { count: 0, max: 1 },
        deps: vec![],
        events: vec![TaskEvent {
            t: now,
            level: "info".to_string(),
            msg: "export task enqueued".to_string(),
        }],
        dedupe_key: None,
    };

    {
        let mut guard = state.inner.lock().await;
        let loaded = guard.as_mut().ok_or("No project loaded")?;
        loaded.project.tasks.push(task.clone());
        loaded.project.rebuild_indexes();
        loaded.dirty = true;
    }

    state.task_notify.notify_one();
    let _ = app_handle.emit("task:updated", serde_json::json!({ "task": task }));

    Ok(serde_json::json!({ "taskId": task_id }))
}

// ============================================================
// Helpers
// ============================================================

fn guess_asset_type(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "flv" | "wmv" => "video".to_string(),
        "mp3" | "wav" | "aac" | "flac" | "ogg" | "wma" => "audio".to_string(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tiff" => "image".to_string(),
        _ => "video".to_string(),
    }
}

// ============================================================
// Tauri Entry
// ============================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let app_state = AppState::new();
    let state_for_protocol = app_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .register_uri_scheme_protocol("media", move |_ctx, request| {
            let state = state_for_protocol.clone();
            let uri = request.uri().to_string();
            let range_header = request
                .headers()
                .get("range")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let (asset_id, prefer_proxy) = parse_media_uri(&uri);

            match serve_media_asset_sync(&state, &asset_id, prefer_proxy, range_header.as_deref()) {
                Ok(resp) => resp,
                Err(e) => tauri::http::Response::builder()
                    .status(500)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(e.into_bytes())
                    .unwrap(),
            }
        })
        .manage(app_state.clone())
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_for_runner = app_state.clone();
            let state_for_saver = app_state.clone();

            // Spawn debounce saver
            tauri::async_runtime::spawn(async move {
                project::io::debounce_saver_loop(state_for_saver).await;
            });

            // Spawn task runner
            tauri::async_runtime::spawn(async move {
                task::runner::task_runner_loop(state_for_runner, handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_project,
            open_project,
            save_project,
            get_project,
            import_assets,
            probe_media,
            read_file_base64,
            task_enqueue,
            task_retry,
            task_cancel,
            task_list,
            timeline_add_clip,
            timeline_move_clip,
            timeline_trim_clip,
            timeline_remove_clip,
            timeline_reorder_clips,
            marker_add,
            marker_update,
            marker_remove,
            update_generation_settings,
            providers_list,
            providers_get,
            providers_upsert,
            providers_delete,
            secrets_set,
            secrets_exists,
            secrets_delete,
            providers_test,
            jimeng_generate_image,
            jimeng_task_status,
            jimeng_credit_balance,
            gen_video_enqueue,
            export_draft,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn parse_media_uri(uri: &str) -> (String, bool) {
    let path = uri
        .strip_prefix("media://localhost/")
        .or_else(|| uri.strip_prefix("media://"))
        .or_else(|| uri.strip_prefix("http://media.localhost/"))
        .or_else(|| uri.strip_prefix("https://media.localhost/"))
        .unwrap_or(uri);

    let (path_part, query) = match path.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path, ""),
    };

    let asset_id = percent_decode(path_part);
    let prefer_proxy = query.contains("proxy=1");

    (asset_id, prefer_proxy)
}

fn percent_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &s[i + 1..i + 3],
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

fn serve_media_asset_sync(
    state: &Arc<AppState>,
    asset_id: &str,
    prefer_proxy: bool,
    range_header: Option<&str>,
) -> Result<tauri::http::Response<Vec<u8>>, String> {
    let guard = state.inner.blocking_lock();
    let loaded = guard.as_ref().ok_or("No project loaded")?;

    let asset = loaded
        .project
        .assets
        .iter()
        .find(|a| a.asset_id == asset_id)
        .ok_or(format!("Asset not found: {}", asset_id))?;

    let file_path = if prefer_proxy {
        asset
            .meta
            .get("proxyUri")
            .and_then(|v| v.as_str())
            .map(|p| loaded.project_dir.join(p))
            .unwrap_or_else(|| loaded.project_dir.join(&asset.path))
    } else {
        loaded.project_dir.join(&asset.path)
    };

    drop(guard);

    let file_bytes = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

    let total_len = file_bytes.len();

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content_type = match ext.as_str() {
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "aac" => "audio/aac",
        "ogg" => "audio/ogg",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    };

    if let Some(range) = range_header {
        let (start, end) = parse_range_header(range, total_len);
        let chunk = file_bytes[start..=end].to_vec();

        tauri::http::Response::builder()
            .status(206)
            .header("Content-Type", content_type)
            .header("Content-Length", chunk.len())
            .header("Content-Range", format!("bytes {}-{}/{}", start, end, total_len))
            .header("Accept-Ranges", "bytes")
            .header("Access-Control-Allow-Origin", "*")
            .body(chunk)
            .map_err(|e| format!("Failed to build response: {}", e))
    } else {
        tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", content_type)
            .header("Content-Length", total_len)
            .header("Accept-Ranges", "bytes")
            .header("Access-Control-Allow-Origin", "*")
            .body(file_bytes)
            .map_err(|e| format!("Failed to build response: {}", e))
    }
}

fn parse_range_header(range: &str, total: usize) -> (usize, usize) {
    let range = range.trim_start_matches("bytes=");
    let parts: Vec<&str> = range.split('-').collect();
    let start = parts
        .first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let end = parts
        .get(1)
        .and_then(|s| if s.is_empty() { None } else { s.parse::<usize>().ok() })
        .unwrap_or(total - 1)
        .min(total - 1);
    (start, end)
}
