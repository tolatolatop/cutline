import { v4 as uuidv4 } from "uuid";
import type { ProjectFile, Timeline, ProjectMeta } from "./project";

export function createDefaultTimeline(): Timeline {
  const timelineId = `tl_${uuidv4()}`;
  return {
    timelineId,
    timebase: { fps: 24, unit: "seconds" },
    tracks: [
      {
        trackId: `trk_v_${uuidv4()}`,
        type: "video",
        name: "Draft Video",
        clips: [],
      },
      {
        trackId: `trk_a_${uuidv4()}`,
        type: "audio",
        name: "Draft Audio",
        clips: [],
      },
      {
        trackId: `trk_t_${uuidv4()}`,
        type: "text",
        name: "Notes / Prompts",
        clips: [],
      },
    ],
  };
}

export function createDefaultProject(name: string): ProjectFile {
  const timeline = createDefaultTimeline();
  const projectId = `proj_${uuidv4()}`;
  const now = new Date().toISOString();

  const project: ProjectMeta = {
    projectId,
    name,
    createdAt: now,
    updatedAt: now,
    settings: {
      fps: 24,
      resolution: { width: 1920, height: 1080 },
      aspectRatio: "16:9",
      sampleRate: 48000,
    },
    paths: {
      workspaceRoot: "./workspace",
      assetsDir: "./workspace/assets",
      cacheDir: "./workspace/cache",
      exportsDir: "./workspace/exports",
    },
    timelineId: timeline.timelineId,
    defaultDraftTrackIds: {
      video: timeline.tracks[0].trackId,
      audio: timeline.tracks[1].trackId,
      text: timeline.tracks[2].trackId,
    },
  };

  return {
    schemaVersion: "0.1",
    project,
    assets: [],
    tasks: [],
    timeline,
    exports: [],
    indexes: {
      assetById: {},
      taskById: {},
    },
  };
}
