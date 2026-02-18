mod asset;
mod media;
mod project;
mod state;
mod task;

use project::model::{
    Asset, DraftTrackIds, Indexes, ProjectFile, ProjectMeta, ProjectPaths, ProjectSettings,
    Resolution, Task, TaskError, TaskEvent, TaskRetries, Timeline, Timebase, Track,
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
        schema_version: "0.1".to_string(),
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
                    clips: vec![],
                },
                Track {
                    track_id: audio_track_id,
                    track_type: "audio".to_string(),
                    name: "Draft Audio".to_string(),
                    clips: vec![],
                },
                Track {
                    track_id: text_track_id,
                    track_type: "text".to_string(),
                    name: "Notes / Prompts".to_string(),
                    clips: vec![],
                },
            ],
        },
        exports: vec![],
        indexes: Indexes {
            asset_by_id: HashMap::new(),
            task_by_id: HashMap::new(),
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
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
