import { useRef, useMemo, useCallback, useEffect } from "react";
import { useProjectStore } from "../store/projectStore";
import {
  useTimelineViewStore,
  msToPixels,
  pixelsToMs,
  formatMs,
  ZOOM_LEVELS,
} from "../store/timelineViewStore";
import { useThumbnail } from "../hooks/useThumbnail";
import * as commands from "../services/commands";
import type { Clip, Asset } from "../models/project";

const TRACK_HEIGHT = 56;
const RULER_HEIGHT = 28;
const HANDLE_WIDTH = 6;
const SNAP_THRESHOLD_PX = 8;
const SNAP_GRID_MS = 100;

function getThumbPath(asset: Asset | undefined): string | null {
  if (!asset) return null;
  const meta = asset.meta as unknown as Record<string, unknown>;
  return (meta?.thumbUri as string) ?? null;
}

function fileName(path: string) {
  return path.split(/[/\\]/).pop() || path;
}

// ============================================================
// TimeRuler
// ============================================================

function TimeRuler({
  totalMs,
  zoomLevel,
  scrollLeftMs,
  onSeek,
}: {
  totalMs: number;
  zoomLevel: number;
  scrollLeftMs: number;
  onSeek: (ms: number) => void;
}) {
  const totalWidth = msToPixels(Math.max(totalMs + 5000, 10000), zoomLevel);
  const tickInterval = zoomLevel >= 150 ? 1000 : zoomLevel >= 75 ? 2000 : 5000;
  const ticks: number[] = [];
  for (let t = 0; t <= totalMs + 5000; t += tickInterval) {
    ticks.push(t);
  }

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const ms = pixelsToMs(x, zoomLevel) + scrollLeftMs;
    onSeek(Math.max(0, Math.round(ms)));
  };

  return (
    <div
      className="relative bg-zinc-900 border-b border-zinc-700 cursor-pointer select-none"
      style={{ height: RULER_HEIGHT, width: totalWidth }}
      onClick={handleClick}
    >
      {ticks.map((t) => {
        const x = msToPixels(t, zoomLevel);
        return (
          <div key={t} className="absolute top-0" style={{ left: x }}>
            <div className="w-px h-3 bg-zinc-600" />
            <span className="absolute top-3 text-[9px] text-zinc-500 font-mono whitespace-nowrap -translate-x-1/2">
              {formatMs(t)}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// ============================================================
// ClipBlock
// ============================================================

function ClipBlock({
  clip,
  asset,
  zoomLevel,
  selected,
  snapTargets,
  onSelect,
}: {
  clip: Clip;
  asset: Asset | undefined;
  zoomLevel: number;
  selected: boolean;
  snapTargets: number[];
  onSelect: () => void;
}) {
  const leftPx = msToPixels(clip.startMs, zoomLevel);
  const widthPx = msToPixels(clip.durationMs, zoomLevel);
  const thumbPath = getThumbPath(asset);
  const thumbUrl = useThumbnail(thumbPath);
  const blockRef = useRef<HTMLDivElement>(null);
  const dragState = useRef<{
    mode: "move" | "trim-left" | "trim-right";
    startX: number;
    origStartMs: number;
    origInMs: number;
    origOutMs: number;
  } | null>(null);

  const snapToGrid = useCallback(
    (ms: number): number => {
      const pxThresh = SNAP_THRESHOLD_PX;
      const msThresh = pixelsToMs(pxThresh, zoomLevel);

      const gridSnapped = Math.round(ms / SNAP_GRID_MS) * SNAP_GRID_MS;
      if (Math.abs(gridSnapped - ms) <= msThresh) return gridSnapped;

      for (const target of snapTargets) {
        if (Math.abs(target - ms) <= msThresh) return target;
      }
      return ms;
    },
    [zoomLevel, snapTargets]
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent, mode: "move" | "trim-left" | "trim-right") => {
      e.stopPropagation();
      e.preventDefault();
      onSelect();

      dragState.current = {
        mode,
        startX: e.clientX,
        origStartMs: clip.startMs,
        origInMs: clip.inMs,
        origOutMs: clip.outMs,
      };

      const el = blockRef.current;
      if (!el) return;

      const onMove = (ev: PointerEvent) => {
        if (!dragState.current) return;
        const dx = ev.clientX - dragState.current.startX;
        const deltaMs = pixelsToMs(dx, zoomLevel);

        if (dragState.current.mode === "move") {
          const newStart = snapToGrid(
            Math.max(0, dragState.current.origStartMs + deltaMs)
          );
          el.style.left = `${msToPixels(newStart, zoomLevel)}px`;
        } else if (dragState.current.mode === "trim-left") {
          const newIn = Math.max(
            0,
            Math.min(
              dragState.current.origOutMs - 100,
              dragState.current.origInMs + deltaMs
            )
          );
          const durMs = dragState.current.origOutMs - newIn;
          const startDelta = newIn - dragState.current.origInMs;
          const newStart = dragState.current.origStartMs + startDelta;
          el.style.left = `${msToPixels(newStart, zoomLevel)}px`;
          el.style.width = `${msToPixels(durMs, zoomLevel)}px`;
        } else if (dragState.current.mode === "trim-right") {
          const newOut = Math.max(
            dragState.current.origInMs + 100,
            dragState.current.origOutMs + deltaMs
          );
          const durMs = newOut - dragState.current.origInMs;
          el.style.width = `${msToPixels(durMs, zoomLevel)}px`;
        }
      };

      const onUp = async (ev: PointerEvent) => {
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);

        if (!dragState.current) return;
        const dx = ev.clientX - dragState.current.startX;
        const deltaMs = pixelsToMs(dx, zoomLevel);

        try {
          if (dragState.current.mode === "move") {
            const newStart = snapToGrid(
              Math.max(0, dragState.current.origStartMs + deltaMs)
            );
            await commands.timelineMoveClip(clip.clipId, Math.round(newStart));
          } else if (dragState.current.mode === "trim-left") {
            const newIn = Math.max(
              0,
              Math.min(
                dragState.current.origOutMs - 100,
                dragState.current.origInMs + deltaMs
              )
            );
            await commands.timelineTrimClip(
              clip.clipId,
              Math.round(newIn),
              undefined
            );
            const startDelta = Math.round(newIn) - dragState.current.origInMs;
            const newStart = dragState.current.origStartMs + startDelta;
            await commands.timelineMoveClip(clip.clipId, Math.round(newStart));
          } else if (dragState.current.mode === "trim-right") {
            const newOut = Math.max(
              dragState.current.origInMs + 100,
              dragState.current.origOutMs + deltaMs
            );
            await commands.timelineTrimClip(
              clip.clipId,
              undefined,
              Math.round(newOut)
            );
          }
        } catch (err) {
          console.error("Clip operation failed:", err);
        }
        dragState.current = null;
      };

      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
    },
    [clip, zoomLevel, onSelect, snapToGrid]
  );

  return (
    <div
      ref={blockRef}
      data-testid={`clip-block-${clip.clipId}`}
      className={`absolute top-1 rounded overflow-hidden cursor-grab active:cursor-grabbing border ${
        selected
          ? "border-blue-500 ring-1 ring-blue-500/50"
          : "border-zinc-600 hover:border-zinc-400"
      }`}
      style={{
        left: leftPx,
        width: Math.max(widthPx, 4),
        height: TRACK_HEIGHT - 8,
      }}
      onPointerDown={(e) => handlePointerDown(e, "move")}
    >
      {/* Left trim handle */}
      <div
        className="absolute left-0 top-0 bottom-0 cursor-col-resize bg-blue-500/0 hover:bg-blue-500/30 z-10"
        style={{ width: HANDLE_WIDTH }}
        onPointerDown={(e) => handlePointerDown(e, "trim-left")}
      />

      {/* Clip content */}
      <div className={`flex items-center gap-1 px-1.5 h-full overflow-hidden ${
        asset?.type === "prompt" ? "bg-amber-900/50" : "bg-zinc-700/80"
      }`}>
        {asset?.type === "prompt" ? (
          <>
            <span className="text-sm shrink-0">📝</span>
            <span className="text-[10px] text-amber-200 truncate">
              {(asset.meta as Record<string, unknown>)?.label as string || fileName(asset.path)}
            </span>
          </>
        ) : (
          <>
            {thumbUrl && (
              <img
                src={thumbUrl}
                className="w-8 h-8 rounded object-cover shrink-0"
                alt=""
              />
            )}
            <span className="text-[10px] text-zinc-300 truncate">
              {asset ? fileName(asset.path) : clip.assetId}
            </span>
          </>
        )}
      </div>

      {/* Right trim handle */}
      <div
        className="absolute right-0 top-0 bottom-0 cursor-col-resize bg-blue-500/0 hover:bg-blue-500/30 z-10"
        style={{ width: HANDLE_WIDTH }}
        onPointerDown={(e) => handlePointerDown(e, "trim-right")}
      />
    </div>
  );
}

// ============================================================
// TrackLane
// ============================================================

function TrackLane({
  trackName,
  clipIds,
  clips,
  assets,
  zoomLevel,
  selectedClipId,
  snapTargets,
  onSelectClip,
}: {
  trackName: string;
  clipIds: string[];
  clips: Record<string, Clip>;
  assets: Asset[];
  zoomLevel: number;
  selectedClipId: string | null;
  snapTargets: number[];
  onSelectClip: (id: string) => void;
}) {
  const assetMap = useMemo(() => {
    const m = new Map<string, Asset>();
    assets.forEach((a) => m.set(a.assetId, a));
    return m;
  }, [assets]);

  return (
    <div data-testid={`track-lane-${trackName}`} className="flex border-b border-zinc-800">
      <div className="w-24 shrink-0 px-2 flex items-center text-[10px] text-zinc-400 bg-zinc-900/50 border-r border-zinc-800">
        {trackName}
      </div>
      <div className="relative flex-1" style={{ height: TRACK_HEIGHT }}>
        {clipIds.map((cid) => {
          const clip = clips[cid];
          if (!clip) return null;
          return (
            <ClipBlock
              key={cid}
              clip={clip}
              asset={assetMap.get(clip.assetId)}
              zoomLevel={zoomLevel}
              selected={selectedClipId === cid}
              snapTargets={snapTargets}
              onSelect={() => onSelectClip(cid)}
            />
          );
        })}
      </div>
    </div>
  );
}

// ============================================================
// PlayheadLine
// ============================================================

function PlayheadLine({
  playheadMs,
  zoomLevel,
}: {
  playheadMs: number;
  zoomLevel: number;
}) {
  const x = msToPixels(playheadMs, zoomLevel);
  return (
    <div
      className="absolute top-0 bottom-0 w-px bg-red-500 z-20 pointer-events-none"
      style={{ left: x }}
    >
      <div className="absolute -top-0 -translate-x-1/2 w-2.5 h-2.5 bg-red-500 rounded-b-sm"
        style={{ clipPath: "polygon(0 0, 100% 0, 50% 100%)" }}
      />
    </div>
  );
}

// ============================================================
// MarkerFlag
// ============================================================

function MarkerFlag({
  tMs,
  label,
  zoomLevel,
  onJump,
}: {
  tMs: number;
  label: string;
  zoomLevel: number;
  onJump: () => void;
}) {
  const x = msToPixels(tMs, zoomLevel);
  return (
    <div
      className="absolute top-0 z-10 cursor-pointer group"
      style={{ left: x }}
      onClick={onJump}
      title={label || formatMs(tMs)}
    >
      <div className="w-2 h-2 bg-yellow-400 rotate-45 -translate-x-1/2" />
      <div className="w-px h-full bg-yellow-400/40 absolute top-2 left-0 -translate-x-1/2" />
    </div>
  );
}

// ============================================================
// Main TimelineView
// ============================================================

export function TimelineView() {
  const { projectFile, selectedAssetId } = useProjectStore();
  const {
    playheadMs,
    zoomLevel,
    scrollLeftMs,
    selectedClipId,
    setPlayhead,
    setZoom,
    selectClip,
  } = useTimelineViewStore();

  const scrollRef = useRef<HTMLDivElement>(null);

  const timeline = projectFile?.timeline;
  const totalMs = timeline?.durationMs ?? 0;

  const snapTargets = useMemo(() => {
    if (!timeline) return [];
    const targets: number[] = [];
    for (const clip of Object.values(timeline.clips)) {
      targets.push(clip.startMs);
      targets.push(clip.startMs + clip.durationMs);
    }
    for (const marker of timeline.markers) {
      targets.push(marker.tMs);
    }
    return targets;
  }, [timeline]);

  const totalWidth = msToPixels(Math.max(totalMs + 5000, 10000), zoomLevel);

  const handleAddClip = useCallback(async () => {
    if (!projectFile || !selectedAssetId) return;
    const asset = projectFile.assets.find((a) => a.assetId === selectedAssetId);
    if (!asset) return;

    const targetTrackType = asset.type === "prompt" ? "text" : asset.type === "audio" ? "audio" : "video";
    const targetTrack = projectFile.timeline.tracks.find(
      (t) => t.type === targetTrackType
    );
    if (!targetTrack) return;

    try {
      await commands.timelineAddClip(targetTrack.trackId, selectedAssetId, playheadMs);
    } catch (err) {
      console.error("Failed to add clip:", err);
    }
  }, [projectFile, selectedAssetId]);

  const handleDeleteClip = useCallback(async () => {
    if (!selectedClipId) return;
    try {
      await commands.timelineRemoveClip(selectedClipId);
      selectClip(null);
    } catch (err) {
      console.error("Failed to remove clip:", err);
    }
  }, [selectedClipId, selectClip]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Delete" || e.key === "Backspace") {
        if (selectedClipId && document.activeElement === document.body) {
          handleDeleteClip();
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedClipId, handleDeleteClip]);

  if (!projectFile) {
    return (
      <div className="flex items-center justify-center h-full text-zinc-500 text-sm">
        打开项目以查看时间轴
      </div>
    );
  }

  return (
    <div data-testid="timeline-view" className="flex flex-col h-full bg-zinc-950">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-zinc-800 bg-zinc-900/50">
        <span className="text-xs text-zinc-400 font-semibold">时间轴</span>

        <button
          data-testid="btn-add-to-timeline"
          onClick={handleAddClip}
          disabled={!selectedAssetId}
          className="px-2 py-0.5 text-[10px] bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 disabled:text-zinc-500 rounded text-white"
        >
          + 添加到时间轴
        </button>

        {selectedClipId && (
          <button
            data-testid="btn-delete-clip"
            onClick={handleDeleteClip}
            className="px-2 py-0.5 text-[10px] bg-red-600/80 hover:bg-red-500 rounded text-white"
          >
            删除 Clip
          </button>
        )}

        <div className="ml-auto flex items-center gap-1">
          <span className="text-[10px] text-zinc-500">缩放:</span>
          {ZOOM_LEVELS.map((z) => (
            <button
              key={z}
              data-testid={`btn-zoom-${z}`}
              onClick={() => setZoom(z)}
              className={`px-1.5 py-0.5 text-[10px] rounded ${
                zoomLevel === z
                  ? "bg-zinc-600 text-zinc-100"
                  : "text-zinc-500 hover:text-zinc-300"
              }`}
            >
              {z}
            </button>
          ))}
        </div>

        <span data-testid="timeline-playhead-time" className="text-[10px] text-zinc-500 font-mono ml-2">
          {formatMs(playheadMs)}
        </span>
      </div>

      {/* Scrollable timeline area */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-auto relative"
        onScroll={(e) => {
          const el = e.currentTarget;
          useTimelineViewStore
            .getState()
            .setScroll(pixelsToMs(el.scrollLeft, zoomLevel));
        }}
      >
        <div style={{ width: totalWidth, minHeight: "100%" }} className="relative">
          {/* Ruler */}
          <TimeRuler
            totalMs={totalMs}
            zoomLevel={zoomLevel}
            scrollLeftMs={scrollLeftMs}
            onSeek={setPlayhead}
          />

          {/* Tracks */}
          <div className="relative" style={{ marginLeft: 0 }}>
            {/* Playhead */}
            <div className="absolute top-0 bottom-0 left-24" style={{ width: totalWidth }}>
              <PlayheadLine playheadMs={playheadMs} zoomLevel={zoomLevel} />
              {timeline?.markers.map((m) => (
                <MarkerFlag
                  key={m.markerId}
                  tMs={m.tMs}
                  label={m.label}
                  zoomLevel={zoomLevel}
                  onJump={() => setPlayhead(m.tMs)}
                />
              ))}
            </div>

            {timeline?.tracks.map((track) => (
              <TrackLane
                key={track.trackId}
                trackName={track.name}
                clipIds={track.clipIds}
                clips={timeline.clips}
                assets={projectFile.assets}
                zoomLevel={zoomLevel}
                selectedClipId={selectedClipId}
                snapTargets={snapTargets}
                onSelectClip={selectClip}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
