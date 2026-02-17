use std::fs;
use std::path::Path;

use super::model::ProjectFile;

pub fn read_project(path: &Path) -> Result<ProjectFile, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("读取 project.json 失败: {}", e))?;
    let pf: ProjectFile =
        serde_json::from_str(&content).map_err(|e| format!("解析 project.json 失败: {}", e))?;
    Ok(pf)
}

pub fn write_project(path: &Path, project: &ProjectFile) -> Result<(), String> {
    let content = serde_json::to_string_pretty(project)
        .map_err(|e| format!("序列化 project.json 失败: {}", e))?;
    fs::write(path, content).map_err(|e| format!("写入 project.json 失败: {}", e))?;
    Ok(())
}

pub fn ensure_workspace_dirs(project_dir: &Path) -> Result<(), String> {
    let dirs = [
        "workspace/assets/video",
        "workspace/assets/audio",
        "workspace/assets/images",
        "workspace/assets/prompts",
        "workspace/cache",
        "workspace/exports",
    ];
    for dir in &dirs {
        let full = project_dir.join(dir);
        fs::create_dir_all(&full)
            .map_err(|e| format!("创建目录 {} 失败: {}", full.display(), e))?;
    }
    Ok(())
}
