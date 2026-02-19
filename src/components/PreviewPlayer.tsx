import { useRef, useEffect, useCallback, useState } from "react";
import { useProjectStore } from "../store/projectStore";
import {
  useTimelineViewStore,
  formatMs,
} from "../store/timelineViewStore";
import * as commands from "../services/commands";
import type { Clip } from "../models/project";

function findClipAtTime(
  clips: Record<string, Clip>,
  trackClipIds: string[],
  playheadMs: number
): Clip | null {
  for (const cid of trackClipIds) {
    const clip = clips[cid];
    if (!clip) continue;
    if (
      playheadMs >= clip.startMs &&
      playheadMs < clip.startMs + clip.durationMs
    ) {
      return clip;
    }
  }
  return null;
}

export function PreviewPlayer() {
  const { projectFile } = useProjectStore();
  const { playheadMs, isPlaying, setPlayhead, pause, togglePlay } =
    useTimelineViewStore();

  const videoRef = useRef<HTMLVideoElement>(null);
  const rafRef = useRef<number>(0);
  const lastTimeRef = useRef<number>(0);
  const currentClipRef = useRef<string | null>(null);
  const [currentSrc, setCurrentSrc] = useState<string>("");
  const [isBlack, setIsBlack] = useState(true);

  const timeline = projectFile?.timeline;
  const durationMs = timeline?.durationMs ?? 0;

  const videoTrack = timeline?.tracks.find((t) => t.type === "video");

  const getAssetMediaUrl = useCallback(
    (assetId: string, preferProxy: boolean) => {
      return `media://localhost/${assetId}${preferProxy ? "?proxy=1" : ""}`;
    },
    []
  );

  const syncVideoToPlayhead = useCallback(
    (ms: number) => {
      if (!timeline || !videoTrack) return;

      const clip = findClipAtTime(
        timeline.clips,
        videoTrack.clipIds,
        ms
      );

      if (!clip) {
        setIsBlack(true);
        if (currentClipRef.current !== null) {
          currentClipRef.current = null;
          setCurrentSrc("");
        }
        return;
      }

      setIsBlack(false);

      if (currentClipRef.current !== clip.clipId) {
        currentClipRef.current = clip.clipId;
        const asset = projectFile?.assets.find(
          (a) => a.assetId === clip.assetId
        );
        const hasProxy = !!(asset?.meta as unknown as Record<string, unknown>)?.proxyUri;
        const url = getAssetMediaUrl(clip.assetId, hasProxy);
        setCurrentSrc(url);
      }

      const video = videoRef.current;
      if (video && video.readyState >= 1) {
        const targetTime = (ms - clip.startMs + clip.inMs) / 1000;
        if (Math.abs(video.currentTime - targetTime) > 0.1) {
          video.currentTime = targetTime;
        }
      }
    },
    [timeline, videoTrack, projectFile, getAssetMediaUrl]
  );

  // Playback RAF loop
  useEffect(() => {
    if (!isPlaying) {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      return;
    }

    lastTimeRef.current = performance.now();

    const tick = (now: number) => {
      const deltaMs = now - lastTimeRef.current;
      lastTimeRef.current = now;

      const newPlayhead =
        useTimelineViewStore.getState().playheadMs + deltaMs;

      if (newPlayhead >= durationMs) {
        setPlayhead(durationMs);
        pause();
        return;
      }

      setPlayhead(newPlayhead);
      syncVideoToPlayhead(newPlayhead);

      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);

    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, [isPlaying, durationMs, setPlayhead, pause, syncVideoToPlayhead]);

  // Sync video when playhead changes externally (seek)
  useEffect(() => {
    if (!isPlaying) {
      syncVideoToPlayhead(playheadMs);

      const video = videoRef.current;
      if (video && !video.paused) {
        video.pause();
      }
    }
  }, [playheadMs, isPlaying, syncVideoToPlayhead]);

  // Play/pause the actual video element
  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    if (isPlaying && currentSrc && !isBlack) {
      video.play().catch(() => {});
    } else {
      video.pause();
    }
  }, [isPlaying, currentSrc, isBlack]);

  const handleSeek = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const ms = Number(e.target.value);
      setPlayhead(ms);
      syncVideoToPlayhead(ms);
    },
    [setPlayhead, syncVideoToPlayhead]
  );

  const handleCaptureFrame = useCallback(async () => {
    if (!timeline || !videoTrack) return;
    const clip = findClipAtTime(
      timeline.clips,
      videoTrack.clipIds,
      playheadMs
    );
    if (!clip) return;

    try {
      await commands.taskEnqueue(
        "capture_frame",
        {
          assetId: clip.assetId,
          tMs: Math.round(playheadMs - clip.startMs + clip.inMs),
          useProxy: true,
        },
        undefined,
        undefined
      );
    } catch (err) {
      console.error("Failed to enqueue capture_frame:", err);
    }
  }, [timeline, videoTrack, playheadMs]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.code === "Space" && document.activeElement === document.body) {
        e.preventDefault();
        togglePlay();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [togglePlay]);

  if (!projectFile) {
    return (
      <div className="flex-1 flex items-center justify-center bg-black text-zinc-500 text-sm">
        打开项目以预览
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-black">
      {/* Video area */}
      <div className="flex-1 flex items-center justify-center relative overflow-hidden">
        {isBlack || !currentSrc ? (
          <div className="text-zinc-600 text-sm">
            {durationMs === 0 ? "时间轴为空" : "无 clip 覆盖"}
          </div>
        ) : (
          <video
            ref={videoRef}
            src={currentSrc}
            className="max-w-full max-h-full"
            muted
            playsInline
          />
        )}
      </div>

      {/* Transport controls */}
      <div className="flex items-center gap-2 px-3 py-1.5 bg-zinc-900/80 border-t border-zinc-800">
        <button
          onClick={togglePlay}
          className="px-2 py-0.5 text-xs bg-zinc-700 hover:bg-zinc-600 rounded text-white"
        >
          {isPlaying ? "⏸ 暂停" : "▶ 播放"}
        </button>

        <input
          type="range"
          min={0}
          max={Math.max(durationMs, 1)}
          value={playheadMs}
          onChange={handleSeek}
          className="flex-1 h-1 accent-blue-500"
        />

        <span className="text-[10px] text-zinc-400 font-mono min-w-[80px] text-right">
          {formatMs(playheadMs)} / {formatMs(durationMs)}
        </span>

        <button
          onClick={handleCaptureFrame}
          className="px-2 py-0.5 text-[10px] bg-zinc-700 hover:bg-zinc-600 rounded text-white"
          title="抓取当前帧"
        >
          📷 抓帧
        </button>
      </div>
    </div>
  );
}
