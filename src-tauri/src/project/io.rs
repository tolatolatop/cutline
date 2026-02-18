use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::model::ProjectFile;
use crate::state::AppState;

pub fn read_project(path: &Path) -> Result<ProjectFile, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("读取 project.json 失败: {}", e))?;
    let pf: ProjectFile =
        serde_json::from_str(&content).map_err(|e| format!("解析 project.json 失败: {}", e))?;
    Ok(pf)
}

pub fn write_project_atomic(path: &Path, project: &ProjectFile) -> Result<(), String> {
    let content = serde_json::to_string_pretty(project)
        .map_err(|e| format!("序列化 project.json 失败: {}", e))?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &content).map_err(|e| format!("写入临时文件失败: {}", e))?;
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp_path, path).map_err(|e| format!("重命名临时文件失败: {}", e))?;
    Ok(())
}

pub fn ensure_workspace_dirs(project_dir: &Path) -> Result<(), String> {
    let dirs = [
        "workspace/assets/video",
        "workspace/assets/audio",
        "workspace/assets/images",
        "workspace/assets/prompts",
        "workspace/cache",
        "workspace/cache/thumbs",
        "workspace/cache/proxy",
        "workspace/exports",
    ];
    for dir in &dirs {
        let full = project_dir.join(dir);
        fs::create_dir_all(&full)
            .map_err(|e| format!("创建目录 {} 失败: {}", full.display(), e))?;
    }
    Ok(())
}

/// Force an immediate save from the in-memory state.
pub async fn force_save(state: &Arc<AppState>) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    if let Some(loaded) = guard.as_mut() {
        loaded.project.rebuild_indexes();
        loaded.project.project.updated_at = chrono::Utc::now().to_rfc3339();
        write_project_atomic(&loaded.json_path, &loaded.project)?;
        loaded.dirty = false;
    }
    Ok(())
}

/// Debounce saver loop — spawned once at app startup.
/// Waits for save_notify, then waits 800ms for more signals before writing.
pub async fn debounce_saver_loop(state: Arc<AppState>) {
    loop {
        state.save_notify.notified().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        let save_result = {
            let mut guard = state.inner.lock().await;
            if let Some(loaded) = guard.as_mut() {
                if loaded.dirty {
                    loaded.project.rebuild_indexes();
                    loaded.project.project.updated_at = chrono::Utc::now().to_rfc3339();
                    let res = write_project_atomic(&loaded.json_path, &loaded.project);
                    if res.is_ok() {
                        loaded.dirty = false;
                    }
                    Some(res)
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(Err(e)) = save_result {
            eprintln!("[debounce_saver] 写盘失败: {}", e);
        }
    }
}
