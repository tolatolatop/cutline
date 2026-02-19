use std::process::Stdio;
use std::sync::Arc;
use tauri::Emitter;
use tokio::process::Command;

use crate::project::model::{TaskError, TaskProgress};
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
