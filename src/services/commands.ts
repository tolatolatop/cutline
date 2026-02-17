import { invoke } from "@tauri-apps/api/core";
import type { ProjectFile, Asset } from "../models/project";

export async function createProject(
  dirPath: string,
  name: string
): Promise<ProjectFile> {
  return invoke("create_project", { dirPath, name });
}

export async function openProject(
  projectJsonPath: string
): Promise<ProjectFile> {
  return invoke("open_project", { projectJsonPath });
}

export async function saveProject(
  projectJsonPath: string,
  projectData: ProjectFile
): Promise<void> {
  return invoke("save_project", { projectJsonPath, projectData });
}

export async function importAssets(
  projectDir: string,
  filePaths: string[]
): Promise<Asset[]> {
  return invoke("import_assets", { projectDir, filePaths });
}

export async function probeMedia(
  filePath: string
): Promise<Record<string, unknown>> {
  return invoke("probe_media", { filePath });
}
