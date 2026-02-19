import { create } from "zustand";

interface TimelineViewState {
  playheadMs: number;
  isPlaying: boolean;
  zoomLevel: number; // px per second (50, 100, 200)
  scrollLeftMs: number;
  selectedClipId: string | null;

  setPlayhead: (ms: number) => void;
  play: () => void;
  pause: () => void;
  togglePlay: () => void;
  setZoom: (level: number) => void;
  setScroll: (ms: number) => void;
  selectClip: (clipId: string | null) => void;
}

export const ZOOM_LEVELS = [50, 100, 200] as const;

export const useTimelineViewStore = create<TimelineViewState>((set) => ({
  playheadMs: 0,
  isPlaying: false,
  zoomLevel: 100,
  scrollLeftMs: 0,
  selectedClipId: null,

  setPlayhead: (ms) => set({ playheadMs: Math.max(0, Math.round(ms)) }),
  play: () => set({ isPlaying: true }),
  pause: () => set({ isPlaying: false }),
  togglePlay: () => set((s) => ({ isPlaying: !s.isPlaying })),
  setZoom: (level) => set({ zoomLevel: level }),
  setScroll: (ms) => set({ scrollLeftMs: Math.max(0, ms) }),
  selectClip: (clipId) => set({ selectedClipId: clipId }),
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
