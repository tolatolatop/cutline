use std::sync::Arc;
use tauri::Emitter;

use crate::project::io;
use crate::state::AppState;
use crate::task::handlers;

/// Single-worker serial task runner loop.
/// Picks the first queued task whose deps are all succeeded, runs it, repeats.
pub async fn task_runner_loop(state: Arc<AppState>, app_handle: tauri::AppHandle) {
    loop {
        state.task_notify.notified().await;
        // Drain all available work before waiting again
        loop {
            let task_info = pick_next_task(&state).await;
            let (task_id, kind, input) = match task_info {
                Some(t) => t,
                None => break,
            };

            // Check if canceled before starting
            {
                let flags = state.cancel_flags.lock().await;
                if flags.contains(&task_id) {
                    mark_canceled(&state, &task_id, &app_handle).await;
                    continue;
                }
            }

            mark_running(&state, &task_id, &app_handle).await;

            let result = handlers::dispatch(&kind, &task_id, &input, &state, &app_handle).await;

            // Check cancel after execution
            {
                let mut flags = state.cancel_flags.lock().await;
                if flags.remove(&task_id) {
                    mark_canceled(&state, &task_id, &app_handle).await;
                    continue;
                }
            }

            if let Some(err) = result.error {
                mark_failed(&state, &task_id, err, &app_handle).await;
            } else {
                mark_succeeded(&state, &task_id, result.output, &app_handle).await;
            }

            // Force save on state transition
            let _ = io::force_save(&state).await;
        }
    }
}

async fn pick_next_task(state: &Arc<AppState>) -> Option<(String, String, serde_json::Value)> {
    let guard = state.inner.lock().await;
    let loaded = guard.as_ref()?;
    let tasks = &loaded.project.tasks;

    for task in tasks {
        if task.state != "queued" {
            continue;
        }
        let deps_met = task.deps.iter().all(|dep_id| {
            tasks.iter().any(|t| t.task_id == *dep_id && t.state == "succeeded")
        });
        if deps_met {
            return Some((task.task_id.clone(), task.kind.clone(), task.input.clone()));
        }
    }
    None
}

async fn mark_running(state: &Arc<AppState>, task_id: &str, app_handle: &tauri::AppHandle) {
    let mut guard = state.inner.lock().await;
    if let Some(loaded) = guard.as_mut() {
        if let Some(task) = loaded.project.tasks.iter_mut().find(|t| t.task_id == task_id) {
            task.state = "running".to_string();
            task.updated_at = chrono::Utc::now().to_rfc3339();
            task.append_event("info", "Task started");
            loaded.dirty = true;
            let snapshot = task.clone();
            drop(guard);
            let _ = app_handle.emit("task:updated", serde_json::json!({ "task": snapshot }));
        }
    }
}

async fn mark_succeeded(
    state: &Arc<AppState>,
    task_id: &str,
    output: Option<serde_json::Value>,
    app_handle: &tauri::AppHandle,
) {
    let mut guard = state.inner.lock().await;
    if let Some(loaded) = guard.as_mut() {
        if let Some(task) = loaded.project.tasks.iter_mut().find(|t| t.task_id == task_id) {
            task.state = "succeeded".to_string();
            task.updated_at = chrono::Utc::now().to_rfc3339();
            task.output = output;
            task.progress = Some(crate::project::model::TaskProgress {
                phase: "done".to_string(),
                percent: Some(100.0),
                message: None,
            });
            task.append_event("info", "Task succeeded");
            loaded.dirty = true;
            let snapshot = task.clone();
            drop(guard);
            let _ = app_handle.emit("task:updated", serde_json::json!({ "task": snapshot }));

            // Also emit asset:updated if the task modified an asset
            let _ = app_handle.emit("project:updated", serde_json::json!({}));
        }
    }
}

async fn mark_failed(
    state: &Arc<AppState>,
    task_id: &str,
    error: crate::project::model::TaskError,
    app_handle: &tauri::AppHandle,
) {
    let mut guard = state.inner.lock().await;
    if let Some(loaded) = guard.as_mut() {
        if let Some(task) = loaded.project.tasks.iter_mut().find(|t| t.task_id == task_id) {
            let msg = format!("Task failed: {} - {}", error.code, error.message);
            task.state = "failed".to_string();
            task.updated_at = chrono::Utc::now().to_rfc3339();
            task.error = Some(error);
            task.append_event("error", &msg);
            loaded.dirty = true;
            let snapshot = task.clone();
            drop(guard);
            let _ = app_handle.emit("task:updated", serde_json::json!({ "task": snapshot }));
        }
    }
}

async fn mark_canceled(state: &Arc<AppState>, task_id: &str, app_handle: &tauri::AppHandle) {
    let mut guard = state.inner.lock().await;
    if let Some(loaded) = guard.as_mut() {
        if let Some(task) = loaded.project.tasks.iter_mut().find(|t| t.task_id == task_id) {
            task.state = "canceled".to_string();
            task.updated_at = chrono::Utc::now().to_rfc3339();
            task.append_event("warn", "Task canceled");
            loaded.dirty = true;
            let snapshot = task.clone();
            drop(guard);
            let _ = app_handle.emit("task:updated", serde_json::json!({ "task": snapshot }));
        }
    }
}
