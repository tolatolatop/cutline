mod asset;
mod media;
mod project;

use project::model::{
    Asset, DraftTrackIds, Indexes, ProjectFile, ProjectMeta, ProjectPaths,
    ProjectSettings, Resolution, Timeline, Timebase, Track,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================
// Tauri Commands
// ============================================================

#[tauri::command]
fn create_project(dir_path: String, name: String) -> Result<ProjectFile, String> {
    let project_dir = Path::new(&dir_path);
    if !project_dir.exists() {
        std::fs::create_dir_all(project_dir)
            .map_err(|e| format!("创建项目目录失败: {}", e))?;
    }

    project::io::ensure_workspace_dirs(project_dir)?;

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
    project::io::write_project(&project_json_path, &pf)?;

    Ok(pf)
}

#[tauri::command]
fn open_project(project_json_path: String) -> Result<ProjectFile, String> {
    let path = Path::new(&project_json_path);
    project::io::read_project(path)
}

#[tauri::command]
fn save_project(project_json_path: String, project_data: ProjectFile) -> Result<(), String> {
    let path = Path::new(&project_json_path);
    let mut pf = project_data;
    pf.rebuild_indexes();
    pf.project.updated_at = chrono::Utc::now().to_rfc3339();
    project::io::write_project(path, &pf)
}

#[tauri::command]
fn import_assets(project_dir: String, file_paths: Vec<String>) -> Result<Vec<Asset>, String> {
    let proj_dir = Path::new(&project_dir);
    let project_json_path = proj_dir.join("project.json");
    let mut pf = project::io::read_project(&project_json_path)?;

    let mut new_assets: Vec<Asset> = Vec::new();

    for file_path_str in &file_paths {
        let source_path = PathBuf::from(file_path_str);
        if !source_path.exists() {
            return Err(format!("文件不存在: {}", file_path_str));
        }

        let fp = asset::fingerprint::compute_file_fingerprint(&source_path)?;

        if asset::registry::find_duplicate(&pf.assets, &fp.value).is_some() {
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

        let dest_dir = proj_dir.join(sub_dir);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("创建目录失败: {}", e))?;

        let dest_path = dest_dir.join(&file_name);

        if !dest_path.exists() {
            std::fs::copy(&source_path, &dest_path)
                .map_err(|e| format!("复制文件失败: {}", e))?;
        }

        let relative_path = format!("{}/{}", sub_dir, file_name);

        let meta = match asset_type.as_str() {
            "video" | "audio" => {
                match media::probe::ffprobe(&dest_path) {
                    Ok(probe_data) => media::probe::extract_video_meta(&probe_data),
                    Err(_) => serde_json::json!({ "kind": asset_type }),
                }
            }
            "image" => media::probe::extract_image_meta(&dest_path),
            _ => serde_json::json!({ "kind": "unknown" }),
        };

        let asset = Asset {
            asset_id: format!("ast_{}_{}", asset_type, uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string()),
            asset_type: asset_type.clone(),
            source: "uploaded".to_string(),
            fingerprint: fp,
            path: relative_path,
            meta,
            generation: None,
            tags: vec!["source".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        pf.assets.push(asset.clone());
        new_assets.push(asset);
    }

    pf.rebuild_indexes();
    pf.project.updated_at = chrono::Utc::now().to_rfc3339();
    project::io::write_project(&project_json_path, &pf)?;

    Ok(new_assets)
}

#[tauri::command]
fn probe_media(file_path: String) -> Result<serde_json::Value, String> {
    let path = Path::new(&file_path);
    let probe_data = media::probe::ffprobe(path)?;
    Ok(media::probe::extract_video_meta(&probe_data))
}

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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            create_project,
            open_project,
            save_project,
            import_assets,
            probe_media,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
