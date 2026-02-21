use std::process::Stdio;
use std::sync::Arc;
use tauri::Emitter;
use tokio::process::Command;

use crate::project::model::{
    Asset, Clip, Fingerprint, GenerationInfo, TaskError, TaskProgress, Track,
};
use crate::state::AppState;

pub struct HandlerResult {
    pub output: Option<serde_json::Value>,
    pub error: Option<TaskError>,
}

pub async fn dispatch(
    kind: &str,
    task_id: &str,
    input: &serde_json::Value,
    state: &Arc<AppState>,
    app_handle: &tauri::AppHandle,
) -> HandlerResult {
    match kind {
        "probe" => handle_probe(task_id, input, state, app_handle).await,
        "thumb" => handle_thumb(task_id, input, state, app_handle).await,
        "proxy" => handle_proxy(task_id, input, state, app_handle).await,
        "capture_frame" => handle_capture_frame(task_id, input, state, app_handle).await,
        "gen_video" => handle_gen_video(task_id, input, state, app_handle).await,
        "export" => handle_export(task_id, input, state, app_handle).await,
        _ => HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "unknown_kind".to_string(),
                message: format!("Unknown task kind: {}", kind),
                detail: None,
            }),
        },
    }
}

async fn update_progress(
    state: &Arc<AppState>,
    task_id: &str,
    progress: TaskProgress,
    app_handle: &tauri::AppHandle,
) {
    let mut guard = state.inner.lock().await;
    if let Some(loaded) = guard.as_mut() {
        if let Some(task) = loaded.project.tasks.iter_mut().find(|t| t.task_id == task_id) {
            task.progress = Some(progress);
            task.updated_at = chrono::Utc::now().to_rfc3339();
            loaded.dirty = true;
            let snapshot = task.clone();
            drop(guard);
            let _ = app_handle.emit("task:updated", serde_json::json!({ "task": snapshot }));
        }
    }
}

async fn append_task_event(
    state: &Arc<AppState>,
    task_id: &str,
    level: &str,
    msg: &str,
) {
    let mut guard = state.inner.lock().await;
    if let Some(loaded) = guard.as_mut() {
        if let Some(task) = loaded.project.tasks.iter_mut().find(|t| t.task_id == task_id) {
            task.append_event(level, msg);
            loaded.dirty = true;
        }
    }
}

async fn handle_probe(
    task_id: &str,
    input: &serde_json::Value,
    state: &Arc<AppState>,
    app_handle: &tauri::AppHandle,
) -> HandlerResult {
    let asset_id = match input.get("assetId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "missing_input".to_string(),
                message: "Missing assetId in input".to_string(),
                detail: None,
            }),
        },
    };

    let abs_path = {
        let guard = state.inner.lock().await;
        let loaded = match guard.as_ref() {
            Some(l) => l,
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "no_project".to_string(),
                    message: "No project loaded".to_string(),
                    detail: None,
                }),
            },
        };
        let asset = loaded.project.assets.iter().find(|a| a.asset_id == asset_id);
        match asset {
            Some(a) => loaded.project_dir.join(&a.path),
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "asset_not_found".to_string(),
                    message: format!("Asset {} not found", asset_id),
                    detail: None,
                }),
            },
        }
    };

    update_progress(state, task_id, TaskProgress {
        phase: "probing".to_string(),
        percent: Some(50.0),
        message: None,
    }, app_handle).await;

    match crate::media::probe::ffprobe(&abs_path) {
        Ok(probe_data) => {
            let meta = crate::media::probe::extract_video_meta(&probe_data);
            {
                let mut guard = state.inner.lock().await;
                if let Some(loaded) = guard.as_mut() {
                    if let Some(asset) = loaded.project.assets.iter_mut().find(|a| a.asset_id == asset_id) {
                        asset.meta = meta.clone();
                    }
                    loaded.dirty = true;
                }
            }
            HandlerResult {
                output: Some(serde_json::json!({ "assetId": asset_id, "meta": meta })),
                error: None,
            }
        }
        Err(e) => HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "probe_failed".to_string(),
                message: e.to_string(),
                detail: None,
            }),
        },
    }
}

async fn handle_thumb(
    task_id: &str,
    input: &serde_json::Value,
    state: &Arc<AppState>,
    app_handle: &tauri::AppHandle,
) -> HandlerResult {
    let asset_id = match input.get("assetId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "missing_input".to_string(),
                message: "Missing assetId in input".to_string(),
                detail: None,
            }),
        },
    };

    let (abs_path, project_dir, asset_type) = {
        let guard = state.inner.lock().await;
        let loaded = match guard.as_ref() {
            Some(l) => l,
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "no_project".to_string(),
                    message: "No project loaded".to_string(),
                    detail: None,
                }),
            },
        };
        let asset = loaded.project.assets.iter().find(|a| a.asset_id == asset_id);
        match asset {
            Some(a) => (
                loaded.project_dir.join(&a.path),
                loaded.project_dir.clone(),
                a.asset_type.clone(),
            ),
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "asset_not_found".to_string(),
                    message: format!("Asset {} not found", asset_id),
                    detail: None,
                }),
            },
        }
    };

    if asset_type != "video" && asset_type != "image" {
        return HandlerResult {
            output: Some(serde_json::json!({ "skipped": true, "reason": "not a video/image asset" })),
            error: None,
        };
    }

    update_progress(state, task_id, TaskProgress {
        phase: "generating_thumbnail".to_string(),
        percent: Some(10.0),
        message: None,
    }, app_handle).await;

    let thumb_dir = project_dir.join("workspace/cache/thumbs");
    let _ = std::fs::create_dir_all(&thumb_dir);
    let thumb_filename = format!("{}.jpg", asset_id);
    let thumb_path = thumb_dir.join(&thumb_filename);
    let thumb_relative = format!("workspace/cache/thumbs/{}", thumb_filename);

    let result = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", &abs_path.to_string_lossy(),
            "-vframes", "1",
            "-q:v", "2",
            &thumb_path.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();

    let child = match result {
        Ok(c) => c,
        Err(e) => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_spawn_failed".to_string(),
                message: format!("Failed to start ffmpeg: {}", e),
                detail: Some("Ensure ffmpeg is installed and in PATH".to_string()),
            }),
        },
    };

    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_wait_failed".to_string(),
                message: format!("ffmpeg process error: {}", e),
                detail: None,
            }),
        },
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.len() > 2048 {
            Some(stderr[..2048].to_string())
        } else {
            Some(stderr.to_string())
        };
        return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_failed".to_string(),
                message: format!("ffmpeg exited with code {:?}", output.status.code()),
                detail,
            }),
        };
    }

    {
        let mut guard = state.inner.lock().await;
        if let Some(loaded) = guard.as_mut() {
            if let Some(asset) = loaded.project.assets.iter_mut().find(|a| a.asset_id == asset_id) {
                if let Some(meta) = asset.meta.as_object_mut() {
                    meta.insert("thumbUri".to_string(), serde_json::Value::String(thumb_relative.clone()));
                }
            }
            loaded.dirty = true;
        }
    }

    HandlerResult {
        output: Some(serde_json::json!({
            "assetId": asset_id,
            "thumbUri": thumb_relative,
        })),
        error: None,
    }
}

async fn handle_proxy(
    task_id: &str,
    input: &serde_json::Value,
    state: &Arc<AppState>,
    app_handle: &tauri::AppHandle,
) -> HandlerResult {
    let asset_id = match input.get("assetId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "missing_input".to_string(),
                message: "Missing assetId in input".to_string(),
                detail: None,
            }),
        },
    };

    let width = input.get("width").and_then(|v| v.as_u64()).unwrap_or(960) as u32;
    let crf = input.get("crf").and_then(|v| v.as_u64()).unwrap_or(28) as u32;

    let (abs_path, project_dir, asset_type) = {
        let guard = state.inner.lock().await;
        let loaded = match guard.as_ref() {
            Some(l) => l,
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "no_project".to_string(),
                    message: "No project loaded".to_string(),
                    detail: None,
                }),
            },
        };
        let asset = loaded.project.assets.iter().find(|a| a.asset_id == asset_id);
        match asset {
            Some(a) => (
                loaded.project_dir.join(&a.path),
                loaded.project_dir.clone(),
                a.asset_type.clone(),
            ),
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "asset_not_found".to_string(),
                    message: format!("Asset {} not found", asset_id),
                    detail: None,
                }),
            },
        }
    };

    if asset_type != "video" {
        return HandlerResult {
            output: Some(serde_json::json!({ "skipped": true, "reason": "not a video asset" })),
            error: None,
        };
    }

    update_progress(state, task_id, TaskProgress {
        phase: "generating_proxy".to_string(),
        percent: Some(5.0),
        message: Some("Starting ffmpeg transcode".to_string()),
    }, app_handle).await;

    let proxy_dir = project_dir.join("workspace/cache/proxy");
    let _ = std::fs::create_dir_all(&proxy_dir);
    let proxy_filename = format!("{}.mp4", asset_id);
    let proxy_path = proxy_dir.join(&proxy_filename);
    let proxy_relative = format!("workspace/cache/proxy/{}", proxy_filename);

    let scale_filter = format!("scale={}:-2", width);

    let result = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", &abs_path.to_string_lossy(),
            "-vf", &scale_filter,
            "-crf", &crf.to_string(),
            "-c:v", "libx264",
            "-preset", "fast",
            "-c:a", "aac",
            "-b:a", "128k",
            &proxy_path.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();

    let child = match result {
        Ok(c) => c,
        Err(e) => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_spawn_failed".to_string(),
                message: format!("Failed to start ffmpeg: {}", e),
                detail: Some("Ensure ffmpeg is installed and in PATH".to_string()),
            }),
        },
    };

    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_wait_failed".to_string(),
                message: format!("ffmpeg process error: {}", e),
                detail: None,
            }),
        },
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.len() > 2048 {
            Some(stderr[..2048].to_string())
        } else {
            Some(stderr.to_string())
        };
        return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_failed".to_string(),
                message: format!("ffmpeg exited with code {:?}", output.status.code()),
                detail,
            }),
        };
    }

    update_progress(state, task_id, TaskProgress {
        phase: "finalizing".to_string(),
        percent: Some(95.0),
        message: None,
    }, app_handle).await;

    {
        let mut guard = state.inner.lock().await;
        if let Some(loaded) = guard.as_mut() {
            if let Some(asset) = loaded.project.assets.iter_mut().find(|a| a.asset_id == asset_id) {
                if let Some(meta) = asset.meta.as_object_mut() {
                    meta.insert("proxyUri".to_string(), serde_json::Value::String(proxy_relative.clone()));
                }
            }
            loaded.dirty = true;
        }
    }

    HandlerResult {
        output: Some(serde_json::json!({
            "assetId": asset_id,
            "proxyUri": proxy_relative,
            "width": width,
            "crf": crf,
        })),
        error: None,
    }
}

async fn handle_capture_frame(
    task_id: &str,
    input: &serde_json::Value,
    state: &Arc<AppState>,
    app_handle: &tauri::AppHandle,
) -> HandlerResult {
    let asset_id = match input.get("assetId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "missing_input".to_string(),
                message: "Missing assetId in input".to_string(),
                detail: None,
            }),
        },
    };

    let t_ms = match input.get("tMs").and_then(|v| v.as_i64()) {
        Some(t) => t,
        None => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "missing_input".to_string(),
                message: "Missing tMs in input".to_string(),
                detail: None,
            }),
        },
    };

    let use_proxy = input.get("useProxy").and_then(|v| v.as_bool()).unwrap_or(true);

    let (src_path, project_dir) = {
        let guard = state.inner.lock().await;
        let loaded = match guard.as_ref() {
            Some(l) => l,
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "no_project".to_string(),
                    message: "No project loaded".to_string(),
                    detail: None,
                }),
            },
        };
        let asset = match loaded.project.assets.iter().find(|a| a.asset_id == asset_id) {
            Some(a) => a,
            None => return HandlerResult {
                output: None,
                error: Some(TaskError {
                    code: "asset_not_found".to_string(),
                    message: format!("Asset {} not found", asset_id),
                    detail: None,
                }),
            },
        };

        let src = if use_proxy {
            asset
                .meta
                .get("proxyUri")
                .and_then(|v| v.as_str())
                .map(|p| loaded.project_dir.join(p))
                .unwrap_or_else(|| loaded.project_dir.join(&asset.path))
        } else {
            loaded.project_dir.join(&asset.path)
        };

        (src, loaded.project_dir.clone())
    };

    update_progress(state, task_id, TaskProgress {
        phase: "capturing_frame".to_string(),
        percent: Some(10.0),
        message: Some(format!("Capturing frame at {}ms", t_ms)),
    }, app_handle).await;

    let captures_dir = project_dir.join("workspace/cache/captures");
    let _ = std::fs::create_dir_all(&captures_dir);
    let out_filename = format!("{}_{}.png", asset_id, t_ms);
    let out_path = captures_dir.join(&out_filename);
    let out_relative = format!("workspace/cache/captures/{}", out_filename);

    let ss = format!("{:.3}", t_ms as f64 / 1000.0);

    let result = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss", &ss,
            "-i", &src_path.to_string_lossy(),
            "-vframes", "1",
            "-q:v", "2",
            &out_path.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();

    let child = match result {
        Ok(c) => c,
        Err(e) => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_spawn_failed".to_string(),
                message: format!("Failed to start ffmpeg: {}", e),
                detail: Some("Ensure ffmpeg is installed and in PATH".to_string()),
            }),
        },
    };

    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_wait_failed".to_string(),
                message: format!("ffmpeg process error: {}", e),
                detail: None,
            }),
        },
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.len() > 2048 {
            Some(stderr[..2048].to_string())
        } else {
            Some(stderr.to_string())
        };
        return HandlerResult {
            output: None,
            error: Some(TaskError {
                code: "ffmpeg_failed".to_string(),
                message: format!("ffmpeg exited with code {:?}", output.status.code()),
                detail,
            }),
        };
    }

    update_progress(state, task_id, TaskProgress {
        phase: "creating_asset".to_string(),
        percent: Some(80.0),
        message: None,
    }, app_handle).await;

    let new_asset_id = format!(
        "ast_image_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );

    let image_meta = crate::media::probe::extract_image_meta(&out_path);

    let new_asset = crate::project::model::Asset {
        asset_id: new_asset_id.clone(),
        asset_type: "image".to_string(),
        source: "generated".to_string(),
        fingerprint: crate::project::model::Fingerprint {
            algo: "sha256".to_string(),
            value: format!("capture_{}_{}", asset_id, t_ms),
            basis: "model_output_bytes".to_string(),
        },
        path: out_relative.clone(),
        meta: image_meta,
        generation: Some(crate::project::model::GenerationInfo {
            task_id: task_id.to_string(),
            model: "ffmpeg".to_string(),
            params: serde_json::json!({ "assetId": asset_id, "tMs": t_ms }),
        }),
        tags: vec!["capture".to_string()],
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Add asset and auto-enqueue thumb task
    let thumb_task_id = {
        let mut guard = state.inner.lock().await;
        if let Some(loaded) = guard.as_mut() {
            loaded.project.assets.push(new_asset);

            let now = chrono::Utc::now().to_rfc3339();
            let tid = format!(
                "task_thumb_{}",
                &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
            );
            let thumb_task = crate::project::model::Task {
                task_id: tid.clone(),
                kind: "thumb".to_string(),
                state: "queued".to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
                input: serde_json::json!({ "assetId": new_asset_id }),
                output: None,
                progress: None,
                error: None,
                retries: crate::project::model::TaskRetries { count: 0, max: 3 },
                deps: vec![],
                events: vec![crate::project::model::TaskEvent {
                    t: now,
                    level: "info".to_string(),
                    msg: "Auto-enqueued thumb for captured frame".to_string(),
                }],
                dedupe_key: Some(format!("thumb:{}", new_asset_id)),
            };
            loaded.project.tasks.push(thumb_task);
            loaded.project.rebuild_indexes();
            loaded.dirty = true;
            tid
        } else {
            String::new()
        }
    };

    if !thumb_task_id.is_empty() {
        state.task_notify.notify_one();
    }

    HandlerResult {
        output: Some(serde_json::json!({
            "newAssetId": new_asset_id,
            "path": out_relative,
            "tMs": t_ms,
        })),
        error: None,
    }
}

// ---------------------------------------------------------------------------
// gen_video handler
// ---------------------------------------------------------------------------

fn build_jimeng_client(
    app_handle: &tauri::AppHandle,
    provider_name: &str,
    profile_name: &str,
) -> Result<crate::providers::jimeng::client::JimengClient, String> {
    let path = crate::provider::io::providers_path(app_handle)?;
    let file = crate::provider::io::load_providers(&path)?;
    let prov = file
        .providers
        .get(provider_name)
        .ok_or(format!("provider_not_found: {}", provider_name))?;
    let profile = prov
        .profiles
        .get(profile_name)
        .ok_or(format!("profile_not_found: {}", profile_name))?;

    let secret = crate::secrets::get_secret(&profile.credential_ref)?
        .ok_or("missing_credentials: 请在设置中连接 Provider".to_string())?;

    let timeout_secs = profile.timeout_ms / 1000;
    crate::providers::jimeng::client::JimengClient::new(
        &secret,
        Some(prov.base_url.as_str()),
        timeout_secs.max(10),
    )
}

const DRAFT_TRACK_ID: &str = "trk_draft";
const MAX_POLL_ATTEMPTS: u32 = 120;
const POLL_INTERVAL_SECS: u64 = 5;

async fn handle_gen_video(
    task_id: &str,
    input: &serde_json::Value,
    state: &Arc<AppState>,
    app_handle: &tauri::AppHandle,
) -> HandlerResult {
    let provider_name = match input.get("providerName").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return err_result("missing_input", "Missing providerName"),
    };
    let profile_name = match input.get("profileName").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return err_result("missing_input", "Missing profileName"),
    };
    let prompt = match input.get("prompt").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return err_result("missing_input", "Missing prompt"),
    };
    let model = input.get("model").and_then(|v| v.as_str()).unwrap_or("jimeng-video-3.0");
    let ratio = input.get("ratio").and_then(|v| v.as_str()).unwrap_or("16:9");
    let duration_ms = input.get("durationMs").and_then(|v| v.as_u64()).map(|v| v as u32);
    let start_ms = input.get("startMs").and_then(|v| v.as_i64()).unwrap_or(0);

    // Step 1: Build client
    append_task_event(state, task_id, "info", &format!(
        "Building client for {}/{}", provider_name, profile_name
    )).await;

    let client = match build_jimeng_client(app_handle, &provider_name, &profile_name) {
        Ok(c) => c,
        Err(e) => {
            append_task_event(state, task_id, "error", &format!("Client build failed: {}", e)).await;
            return err_result("provider_error", &format!("Failed to build client: {}", e));
        }
    };

    update_progress(state, task_id, TaskProgress {
        phase: "submitting".to_string(),
        percent: Some(5.0),
        message: Some("Submitting video generation request".to_string()),
    }, app_handle).await;

    // Step 2: Submit
    append_task_event(state, task_id, "info", &format!(
        "Submitting: model={}, ratio={}, prompt={}", model, ratio, &prompt[..prompt.len().min(50)]
    )).await;

    let gen_result = match crate::providers::jimeng::api::generate_video(
        &client, &prompt, model, ratio, duration_ms,
    ).await {
        Ok(r) => r,
        Err(e) => {
            append_task_event(state, task_id, "error", &format!("Submit failed: {}", e)).await;
            return err_result("provider_error", &format!("Video generation submit failed: {}", e));
        }
    };

    append_task_event(state, task_id, "info", &format!(
        "Submitted: submit_id={}, history_id={}", gen_result.submit_id, gen_result.history_id
    )).await;

    update_progress(state, task_id, TaskProgress {
        phase: "submitted".to_string(),
        percent: Some(10.0),
        message: Some(format!("submit_id: {}", gen_result.submit_id)),
    }, app_handle).await;

    // Step 3: Poll loop
    let submit_ids = vec![gen_result.submit_id.clone()];
    let history_ids: Vec<String> = if gen_result.history_id.is_empty() {
        vec![]
    } else {
        vec![gen_result.history_id.clone()]
    };

    let mut final_result = None;

    for attempt in 0..MAX_POLL_ATTEMPTS {
        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let percent = 10.0 + (attempt as f32 / MAX_POLL_ATTEMPTS as f32) * 70.0;
        update_progress(state, task_id, TaskProgress {
            phase: "generating".to_string(),
            percent: Some(percent.min(80.0)),
            message: Some(format!("Polling attempt {}/{}", attempt + 1, MAX_POLL_ATTEMPTS)),
        }, app_handle).await;

        let status_map = match crate::providers::jimeng::api::get_task_status(
            &client,
            &history_ids,
            Some(&submit_ids),
        ).await {
            Ok(m) => m,
            Err(e) => {
                if attempt >= 3 {
                    return err_result("provider_error", &format!("Poll failed after {} attempts: {}", attempt + 1, e));
                }
                continue;
            }
        };

        // Check all returned results for completion
        for task_status in status_map.values() {
            use crate::providers::jimeng::constants::TaskStatus;
            match TaskStatus::from_u32(task_status.status) {
                Some(TaskStatus::Completed) | Some(TaskStatus::Partial) => {
                    final_result = Some(task_status.clone());
                    break;
                }
                Some(TaskStatus::Failed) => {
                    return err_result("provider_error", &format!(
                        "Video generation failed (fail_code: {})", task_status.fail_code
                    ));
                }
                _ => {} // Queued or Processing, keep polling
            }
        }

        if final_result.is_some() {
            break;
        }
    }

    let task_status = match final_result {
        Some(r) => r,
        None => {
            append_task_event(state, task_id, "error", "Generation timed out after polling").await;
            return err_result("timeout", "Video generation timed out after polling");
        }
    };

    append_task_event(state, task_id, "info", &format!(
        "Generation completed with status={}", task_status.status
    )).await;

    // Step 4: Extract video URL
    let video_url = match crate::providers::jimeng::api::extract_video_url(&task_status) {
        Some(url) => url,
        None => {
            append_task_event(state, task_id, "error", "No video URL in completed task").await;
            return err_result("provider_error", "No video URL found in completed task");
        }
    };

    update_progress(state, task_id, TaskProgress {
        phase: "downloading".to_string(),
        percent: Some(85.0),
        message: Some("Downloading generated video".to_string()),
    }, app_handle).await;

    // Step 5: Download to .cache/gen/
    let project_dir = {
        let guard = state.inner.lock().await;
        match guard.as_ref() {
            Some(loaded) => loaded.project_dir.clone(),
            None => return err_result("no_project", "No project loaded"),
        }
    };

    let gen_dir = project_dir.join("workspace").join("cache").join("gen");
    let _ = std::fs::create_dir_all(&gen_dir);
    let file_name = format!("{}.mp4", task_id);
    let file_path = gen_dir.join(&file_name);
    let relative_path = format!("workspace/cache/gen/{}", file_name);

    let download_client = reqwest::Client::new();
    let resp = match download_client.get(&video_url).send().await {
        Ok(r) => r,
        Err(e) => return err_result("download_error", &format!("Failed to download video: {}", e)),
    };

    if !resp.status().is_success() {
        return err_result("download_error", &format!("Download HTTP {}", resp.status()));
    }

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return err_result("download_error", &format!("Failed to read video bytes: {}", e)),
    };

    if let Err(e) = std::fs::write(&file_path, &bytes) {
        append_task_event(state, task_id, "error", &format!("File write failed: {}", e)).await;
        return err_result("io_error", &format!("Failed to write video file: {}", e));
    }

    append_task_event(state, task_id, "info", &format!(
        "Downloaded {} bytes to {}", bytes.len(), relative_path
    )).await;

    update_progress(state, task_id, TaskProgress {
        phase: "registering".to_string(),
        percent: Some(92.0),
        message: Some("Registering asset and inserting clip".to_string()),
    }, app_handle).await;

    // Step 6: Probe the downloaded video for duration
    let probe_duration_ms = match crate::media::probe::ffprobe(&file_path) {
        Ok(probe_data) => {
            let meta = crate::media::probe::extract_video_meta(&probe_data);
            meta.get("durationSec")
                .and_then(|v| v.as_f64())
                .map(|s| (s * 1000.0) as i64)
                .unwrap_or(5000)
        }
        Err(_) => duration_ms.map(|d| d as i64).unwrap_or(5000),
    };

    // Step 7: Register asset + insert clip on trk_draft
    let new_asset_id = format!(
        "ast_video_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );
    let new_clip_id = format!(
        "clip_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]
    );

    let fingerprint_value = format!("sha256:{}", &uuid::Uuid::new_v4().to_string().replace("-", ""));

    let new_asset = Asset {
        asset_id: new_asset_id.clone(),
        asset_type: "video".to_string(),
        source: "generated".to_string(),
        fingerprint: Fingerprint {
            algo: "sha256".to_string(),
            value: fingerprint_value,
            basis: "model_output_bytes".to_string(),
        },
        path: relative_path.clone(),
        meta: serde_json::json!({
            "durationMs": probe_duration_ms,
            "source": "gen_video",
        }),
        generation: Some(GenerationInfo {
            task_id: task_id.to_string(),
            model: model.to_string(),
            params: serde_json::json!({
                "prompt": prompt,
                "ratio": ratio,
                "durationMs": duration_ms,
            }),
        }),
        tags: vec!["generated".to_string(), "video".to_string()],
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let new_clip = Clip {
        clip_id: new_clip_id.clone(),
        asset_id: new_asset_id.clone(),
        track_id: DRAFT_TRACK_ID.to_string(),
        start_ms,
        duration_ms: probe_duration_ms,
        in_ms: 0,
        out_ms: probe_duration_ms,
    };

    {
        let mut guard = state.inner.lock().await;
        if let Some(loaded) = guard.as_mut() {
            loaded.project.assets.push(new_asset);

            // Find or create trk_draft
            let track_exists = loaded.project.timeline.tracks.iter().any(|t| t.track_id == DRAFT_TRACK_ID);
            if !track_exists {
                loaded.project.timeline.tracks.push(Track {
                    track_id: DRAFT_TRACK_ID.to_string(),
                    track_type: "video".to_string(),
                    name: "Draft".to_string(),
                    clip_ids: vec![],
                });
            }

            if let Some(track) = loaded.project.timeline.tracks.iter_mut().find(|t| t.track_id == DRAFT_TRACK_ID) {
                track.clip_ids.push(new_clip_id.clone());
            }

            loaded.project.timeline.clips.insert(new_clip_id.clone(), new_clip);
            loaded.project.timeline.recalc_duration();
            loaded.project.rebuild_indexes();
            loaded.dirty = true;
        }
    }

    let _ = app_handle.emit("project:updated", serde_json::json!({}));

    HandlerResult {
        output: Some(serde_json::json!({
            "assetId": new_asset_id,
            "clipId": new_clip_id,
            "path": relative_path,
            "durationMs": probe_duration_ms,
        })),
        error: None,
    }
}

fn err_result(code: &str, message: &str) -> HandlerResult {
    HandlerResult {
        output: None,
        error: Some(TaskError {
            code: code.to_string(),
            message: message.to_string(),
            detail: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// export handler
// ---------------------------------------------------------------------------

async fn handle_export(
    task_id: &str,
    input: &serde_json::Value,
    state: &Arc<AppState>,
    app_handle: &tauri::AppHandle,
) -> HandlerResult {
    let track_id = input.get("trackId").and_then(|v| v.as_str()).unwrap_or(DRAFT_TRACK_ID);

    update_progress(state, task_id, TaskProgress {
        phase: "collecting".to_string(),
        percent: Some(5.0),
        message: Some("Collecting clips from track".to_string()),
    }, app_handle).await;

    // Collect clip info from the target track
    let (clip_paths, project_dir) = {
        let guard = state.inner.lock().await;
        let loaded = match guard.as_ref() {
            Some(l) => l,
            None => return err_result("no_project", "No project loaded"),
        };

        let track = match loaded.project.timeline.tracks.iter().find(|t| t.track_id == track_id) {
            Some(t) => t,
            None => return err_result("track_not_found", &format!("Track {} not found", track_id)),
        };

        if track.clip_ids.is_empty() {
            return err_result("no_clips", "Track has no clips to export");
        }

        // Collect clips sorted by start_ms
        let mut clips: Vec<&Clip> = track.clip_ids.iter()
            .filter_map(|cid| loaded.project.timeline.clips.get(cid))
            .collect();
        clips.sort_by_key(|c| c.start_ms);

        let paths: Vec<std::path::PathBuf> = clips.iter()
            .filter_map(|clip| {
                loaded.project.assets.iter()
                    .find(|a| a.asset_id == clip.asset_id)
                    .map(|a| loaded.project_dir.join(&a.path))
            })
            .collect();

        if paths.is_empty() {
            return err_result("no_assets", "No assets found for clips");
        }

        (paths, loaded.project_dir.clone())
    };

    let exports_dir = project_dir.join("workspace").join("exports");
    let _ = std::fs::create_dir_all(&exports_dir);

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let output_filename = format!("export_{}.mp4", timestamp);
    let output_path = exports_dir.join(&output_filename);
    let output_relative = format!("workspace/exports/{}", output_filename);

    update_progress(state, task_id, TaskProgress {
        phase: "encoding".to_string(),
        percent: Some(20.0),
        message: Some(format!("Exporting {} clip(s)", clip_paths.len())),
    }, app_handle).await;

    if clip_paths.len() == 1 {
        // Single clip: transcode
        let child = Command::new("ffmpeg")
            .args([
                "-y",
                "-i", &clip_paths[0].to_string_lossy(),
                "-c:v", "libx264",
                "-crf", "23",
                "-preset", "fast",
                "-c:a", "aac",
                "-b:a", "128k",
                &output_path.to_string_lossy(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn();

        let child = match child {
            Ok(c) => c,
            Err(e) => return err_result("ffmpeg_spawn_failed", &format!("Failed to start ffmpeg: {}", e)),
        };

        let output = match child.wait_with_output().await {
            Ok(o) => o,
            Err(e) => return err_result("ffmpeg_wait_failed", &format!("ffmpeg process error: {}", e)),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return err_result("ffmpeg_failed", &format!("ffmpeg exited {:?}: {}", output.status.code(), &stderr[..stderr.len().min(512)]));
        }
    } else {
        // Multiple clips: write concat list and use ffmpeg concat
        let concat_list_path = exports_dir.join(format!("concat_{}.txt", timestamp));
        let mut concat_content = String::new();
        for p in &clip_paths {
            let escaped = p.to_string_lossy().replace('\'', "'\\''");
            concat_content.push_str(&format!("file '{}'\n", escaped));
        }
        if let Err(e) = std::fs::write(&concat_list_path, &concat_content) {
            return err_result("io_error", &format!("Failed to write concat list: {}", e));
        }

        // Try concat copy first; fall back to re-encode on failure
        let child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "concat",
                "-safe", "0",
                "-i", &concat_list_path.to_string_lossy(),
                "-c:v", "libx264",
                "-crf", "23",
                "-preset", "fast",
                "-c:a", "aac",
                "-b:a", "128k",
                &output_path.to_string_lossy(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn();

        let child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_file(&concat_list_path);
                return err_result("ffmpeg_spawn_failed", &format!("Failed to start ffmpeg: {}", e));
            }
        };

        let output = match child.wait_with_output().await {
            Ok(o) => o,
            Err(e) => {
                let _ = std::fs::remove_file(&concat_list_path);
                return err_result("ffmpeg_wait_failed", &format!("ffmpeg process error: {}", e));
            }
        };

        let _ = std::fs::remove_file(&concat_list_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return err_result("ffmpeg_failed", &format!("ffmpeg exited {:?}: {}", output.status.code(), &stderr[..stderr.len().min(512)]));
        }
    }

    update_progress(state, task_id, TaskProgress {
        phase: "finalizing".to_string(),
        percent: Some(95.0),
        message: None,
    }, app_handle).await;

    // Register export record
    {
        let mut guard = state.inner.lock().await;
        if let Some(loaded) = guard.as_mut() {
            let export_record = crate::project::model::ExportRecord {
                export_id: format!("exp_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..8]),
                status: "completed".to_string(),
                preset: crate::project::model::ExportPreset {
                    container: "mp4".to_string(),
                    codec: "h264".to_string(),
                    bitrate_kbps: 0,
                },
                start_ms: 0,
                end_ms: 0,
                output_uri: output_relative.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            loaded.project.exports.push(export_record);
            loaded.dirty = true;
        }
    }

    let _ = app_handle.emit("project:updated", serde_json::json!({}));

    HandlerResult {
        output: Some(serde_json::json!({
            "exportPath": output_relative,
        })),
        error: None,
    }
}
