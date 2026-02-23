import { create } from "zustand";
import type { Clip } from "../models/project";

interface TimelineViewState {
  playheadMs: number;
  isPlaying: boolean;
  zoomLevel: number; // px per second (50, 100, 200)
  scrollLeftMs: number;
  selectedClipIds: Set<string>;

  setPlayhead: (ms: number) => void;
  play: () => void;
  pause: () => void;
  togglePlay: () => void;
  setZoom: (level: number) => void;
  setScroll: (ms: number) => void;
  selectClip: (clipId: string | null) => void;
  toggleClip: (clipId: string) => void;
  selectClips: (clipIds: string[]) => void;
  addClips: (clipIds: string[]) => void;
  selectRange: (startMs: number, endMs: number, clips: Record<string, Clip>) => void;
  clearSelection: () => void;
}

export const ZOOM_LEVELS = [50, 100, 200] as const;

export const useTimelineViewStore = create<TimelineViewState>((set) => ({
  playheadMs: 0,
  isPlaying: false,
  zoomLevel: 100,
  scrollLeftMs: 0,
  selectedClipIds: new Set(),

  setPlayhead: (ms) => set({ playheadMs: Math.max(0, Math.round(ms)) }),
  play: () => set({ isPlaying: true }),
  pause: () => set({ isPlaying: false }),
  togglePlay: () => set((s) => ({ isPlaying: !s.isPlaying })),
  setZoom: (level) => set({ zoomLevel: level }),
  setScroll: (ms) => set({ scrollLeftMs: Math.max(0, ms) }),

  selectClip: (clipId) =>
    set({ selectedClipIds: clipId ? new Set([clipId]) : new Set() }),

  toggleClip: (clipId) =>
    set((s) => {
      const next = new Set(s.selectedClipIds);
      if (next.has(clipId)) {
        next.delete(clipId);
      } else {
        next.add(clipId);
      }
      return { selectedClipIds: next };
    }),

  selectClips: (clipIds) => set({ selectedClipIds: new Set(clipIds) }),

  addClips: (clipIds) =>
    set((s) => {
      const next = new Set(s.selectedClipIds);
      clipIds.forEach((id) => next.add(id));
      return { selectedClipIds: next };
    }),

  selectRange: (startMs, endMs, clips) => {
    const lo = Math.min(startMs, endMs);
    const hi = Math.max(startMs, endMs);
    const ids: string[] = [];
    for (const [cid, clip] of Object.entries(clips)) {
      const clipEnd = clip.startMs + clip.durationMs;
      if (clip.startMs < hi && clipEnd > lo) {
        ids.push(cid);
      }
    }
    set({ selectedClipIds: new Set(ids) });
  },

  clearSelection: () => set({ selectedClipIds: new Set() }),
}));

export function msToPixels(ms: number, zoomLevel: number): number {
  return (ms / 1000) * zoomLevel;
}

export function pixelsToMs(px: number, zoomLevel: number): number {
  return (px / zoomLevel) * 1000;
}

export function formatMs(ms: number): string {
  const rounded = Math.round(ms);
  const totalSec = Math.floor(rounded / 1000);
  const minutes = Math.floor(totalSec / 60);
  const seconds = totalSec % 60;
  const centis = Math.floor((rounded % 1000) / 10);
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}.${String(centis).padStart(2, "0")}`;
}
