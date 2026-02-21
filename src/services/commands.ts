import { invoke } from "@tauri-apps/api/core";
import type { ProjectFile, Asset, Clip, Marker, TaskSummary } from "../models/project";

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

// ============================================================
// Timeline Commands
// ============================================================

export async function timelineAddClip(
  trackId: string,
  assetId: string,
  startMs: number
): Promise<Clip> {
  return invoke("timeline_add_clip", { trackId, assetId, startMs });
}

export async function timelineMoveClip(
  clipId: string,
  newStartMs: number
): Promise<void> {
  return invoke("timeline_move_clip", { clipId, newStartMs });
}

export async function timelineTrimClip(
  clipId: string,
  inMs?: number,
  outMs?: number
): Promise<void> {
  return invoke("timeline_trim_clip", { clipId, inMs, outMs });
}

export async function timelineRemoveClip(clipId: string): Promise<void> {
  return invoke("timeline_remove_clip", { clipId });
}

export async function timelineReorderClips(
  trackId: string,
  clipIds: string[]
): Promise<void> {
  return invoke("timeline_reorder_clips", { trackId, clipIds });
}

// ============================================================
// Marker Commands
// ============================================================

export async function markerAdd(
  tMs: number,
  label?: string,
  promptText?: string
): Promise<Marker> {
  return invoke("marker_add", { tMs, label, promptText });
}

export async function markerUpdate(
  markerId: string,
  label?: string,
  promptText?: string,
  tMs?: number
): Promise<void> {
  return invoke("marker_update", { markerId, label, promptText, tMs });
}

export async function markerRemove(markerId: string): Promise<void> {
  return invoke("marker_remove", { markerId });
}

// ============================================================
// Project Settings Commands
// ============================================================

export async function updateGenerationSettings(
  videoProvider?: string,
  videoProfile?: string
): Promise<void> {
  return invoke("update_generation_settings", { videoProvider, videoProfile });
}

// ============================================================
// Generation / Export Commands
// ============================================================

export interface GenVideoParams {
  providerName: string;
  profileName: string;
  prompt: string;
  model?: string;
  ratio?: string;
  durationMs?: number;
  startMs?: number;
}

export async function genVideoEnqueue(
  params: GenVideoParams
): Promise<{ taskId: string }> {
  return invoke("gen_video_enqueue", { ...params });
}

export async function exportDraft(
  trackId?: string
): Promise<{ taskId: string }> {
  return invoke("export_draft", { trackId });
}
