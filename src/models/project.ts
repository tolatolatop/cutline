// ============================================================
// Cutline Project JSON v0 — 完整类型定义
// project.json 只存元数据和相对路径引用，
// 所有资产数据本体以文件形式存储在 workspace 目录中。
// ============================================================

// --- 顶层 ---
export interface ProjectFile {
  schemaVersion: string;
  project: ProjectMeta;
  assets: Asset[];
  tasks: Task[];
  timeline: Timeline;
  exports: ExportRecord[];
  indexes: Indexes;
}

// --- 项目元信息 ---
export interface ProjectMeta {
  projectId: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  settings: ProjectSettings;
  paths: ProjectPaths;
  timelineId: string;
  defaultDraftTrackIds: {
    video: string;
    audio: string;
    text: string;
  };
}

export interface ProjectSettings {
  fps: number;
  resolution: { width: number; height: number };
  aspectRatio: string;
  sampleRate: number;
}

export interface ProjectPaths {
  workspaceRoot: string;
  assetsDir: string;
  cacheDir: string;
  exportsDir: string;
}

// --- 资产（仅元数据 + 相对路径，不含数据本体）---
export type AssetType = "video" | "audio" | "image" | "prompt";
export type AssetSource = "uploaded" | "generated" | "authored";
export type FingerprintBasis = "file_bytes" | "content_json" | "model_output_bytes";

export interface Fingerprint {
  algo: "sha256";
  value: string;
  basis: FingerprintBasis;
}

export interface Asset {
  assetId: string;
  type: AssetType;
  source: AssetSource;
  fingerprint: Fingerprint;
  path: string;
  meta: VideoMeta | AudioMeta | ImageMeta | PromptMeta;
  generation?: GenerationInfo;
  tags: string[];
  createdAt: string;
}

// --- Meta 子类型 ---
export interface VideoMeta {
  kind: "video";
  container: string;
  codec: string;
  durationSec: number;
  width: number;
  height: number;
  fps: number;
  audio?: {
    present: boolean;
    sampleRate: number;
    channels: number;
  };
}

export interface AudioMeta {
  kind: "audio";
  codec: string;
  durationSec: number;
  sampleRate: number;
  channels: number;
}

export interface ImageMeta {
  kind: "image";
  format: string;
  width: number;
  height: number;
}

export interface PromptMeta {
  kind: "prompt";
  language: string;
  format: string;
}

// --- AI 生成信息 ---
export interface GenerationInfo {
  taskId: string;
  model: string;
  params: Record<string, unknown>;
}

// --- 任务 v1 ---
export type TaskKind = "probe" | "thumb" | "proxy" | "generate" | "export";
export type TaskState = "queued" | "running" | "succeeded" | "failed" | "canceled";

export interface TaskProgress {
  phase: string;
  percent?: number;
  message?: string;
}

export interface TaskError {
  code: string;
  message: string;
  detail?: string;
}

export interface TaskRetries {
  count: number;
  max: number;
}

export interface TaskEvent {
  t: string;
  level: "info" | "warn" | "error";
  msg: string;
}

export interface Task {
  taskId: string;
  kind: TaskKind;
  state: TaskState;
  createdAt: string;
  updatedAt: string;
  input: Record<string, unknown>;
  output?: Record<string, unknown>;
  progress?: TaskProgress;
  error?: TaskError;
  retries: TaskRetries;
  deps: string[];
  events: TaskEvent[];
  dedupeKey?: string;
}

export interface TaskSummary {
  taskId: string;
  kind: TaskKind;
  state: TaskState;
  createdAt: string;
  updatedAt: string;
  progress?: TaskProgress;
  error?: TaskError;
  retries: TaskRetries;
}

// --- 时间轴 ---
export interface Timeline {
  timelineId: string;
  timebase: { fps: number; unit: string };
  tracks: Track[];
}

export type TrackType = "video" | "audio" | "text";

export interface Track {
  trackId: string;
  type: TrackType;
  name: string;
  clips: Clip[];
}

export interface Clip {
  clipId: string;
  assetId: string;
  range: { start: number; end: number };
  offset: number;
  segmentIndex?: number;
  promptContextAssetId?: string;
  flags: Record<string, boolean>;
}

// --- 导出 ---
export interface ExportRecord {
  exportId: string;
  status: "planned" | "running" | "completed" | "failed";
  preset: {
    container: string;
    codec: string;
    bitrateKbps: number;
  };
  range: { start: number; end: number };
  outputUri: string;
  createdAt: string;
}

// --- 索引 ---
export interface Indexes {
  assetById: Record<string, number>;
  taskById: Record<string, number>;
}
