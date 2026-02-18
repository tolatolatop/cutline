import { invoke } from "@tauri-apps/api/core";
import type { ProjectFile, Asset, TaskSummary } from "../models/project";

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

export async function saveProject(): Promise<void> {
  return invoke("save_project");
}

export async function getProject(): Promise<ProjectFile> {
  return invoke("get_project");
}

export async function importAssets(
  filePaths: string[]
): Promise<Asset[]> {
  return invoke("import_assets", { filePaths });
}

export async function probeMedia(
  filePath: string
): Promise<Record<string, unknown>> {
  return invoke("probe_media", { filePath });
}

export async function taskEnqueue(
  kind: string,
  input: Record<string, unknown>,
  deps?: string[],
  dedupeKey?: string
): Promise<string> {
  return invoke("task_enqueue", { kind, input, deps, dedupeKey });
}

export async function taskRetry(taskId: string): Promise<void> {
  return invoke("task_retry", { taskId });
}

export async function taskCancel(taskId: string): Promise<void> {
  return invoke("task_cancel", { taskId });
}

export async function taskList(): Promise<TaskSummary[]> {
  return invoke("task_list");
}

export async function readFileBase64(
  relativePath: string
): Promise<string> {
  return invoke("read_file_base64", { relativePath });
}
