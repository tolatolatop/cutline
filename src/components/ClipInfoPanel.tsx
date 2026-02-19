import { useState, useCallback, useEffect } from "react";
import { useProjectStore } from "../store/projectStore";
import { useTimelineViewStore, formatMs } from "../store/timelineViewStore";
import * as commands from "../services/commands";
import type { Clip } from "../models/project";

function fileName(path: string) {
  return path.split(/[/\\]/).pop() || path;
}

export function ClipInfoPanel() {
  const { projectFile } = useProjectStore();
  const { selectedClipId } = useTimelineViewStore();
  const [editStartMs, setEditStartMs] = useState("");
  const [editInMs, setEditInMs] = useState("");
  const [editOutMs, setEditOutMs] = useState("");

  const clip: Clip | undefined = selectedClipId
    ? projectFile?.timeline?.clips?.[selectedClipId]
    : undefined;

  const asset = clip
    ? projectFile?.assets.find((a) => a.assetId === clip.assetId)
    : undefined;

  useEffect(() => {
    if (clip) {
      setEditStartMs(String(clip.startMs));
      setEditInMs(String(clip.inMs));
      setEditOutMs(String(clip.outMs));
    }
  }, [clip]);

  const handleApply = useCallback(async () => {
    if (!clip) return;
    try {
      const newStart = parseInt(editStartMs, 10);
      const newIn = parseInt(editInMs, 10);
      const newOut = parseInt(editOutMs, 10);

      if (!isNaN(newIn) && !isNaN(newOut) && (newIn !== clip.inMs || newOut !== clip.outMs)) {
        await commands.timelineTrimClip(clip.clipId, newIn, newOut);
      }
      if (!isNaN(newStart) && newStart !== clip.startMs) {
        await commands.timelineMoveClip(clip.clipId, newStart);
      }
    } catch (err) {
      console.error("Failed to update clip:", err);
    }
  }, [clip, editStartMs, editInMs, editOutMs]);

  if (!clip) {
    return (
      <div className="text-[10px] text-zinc-500 px-3 py-2">
        选择一个 clip 查看属性
      </div>
    );
  }

  return (
    <div data-testid="clip-info-panel" className="px-3 py-2 space-y-2 text-xs">
      <div className="text-zinc-400 font-semibold text-[10px]">Clip 属性</div>

      {asset && (
        <div className="text-zinc-300 text-[10px] truncate">
          {fileName(asset.path)}
        </div>
      )}

      <div className="text-[10px] text-zinc-500">
        时长: {formatMs(clip.durationMs)}
      </div>

      <div className="space-y-1">
        <label className="block text-[10px] text-zinc-500">
          起始 (ms)
          <input
            data-testid="clip-edit-start"
            type="number"
            value={editStartMs}
            onChange={(e) => setEditStartMs(e.target.value)}
            className="w-full mt-0.5 px-1.5 py-0.5 bg-zinc-800 border border-zinc-700 rounded text-zinc-200 text-[10px] focus:outline-none focus:border-zinc-500"
          />
        </label>

        <label className="block text-[10px] text-zinc-500">
          入点 (ms)
          <input
            data-testid="clip-edit-in"
            type="number"
            value={editInMs}
            onChange={(e) => setEditInMs(e.target.value)}
            className="w-full mt-0.5 px-1.5 py-0.5 bg-zinc-800 border border-zinc-700 rounded text-zinc-200 text-[10px] focus:outline-none focus:border-zinc-500"
          />
        </label>

        <label className="block text-[10px] text-zinc-500">
          出点 (ms)
          <input
            data-testid="clip-edit-out"
            type="number"
            value={editOutMs}
            onChange={(e) => setEditOutMs(e.target.value)}
            className="w-full mt-0.5 px-1.5 py-0.5 bg-zinc-800 border border-zinc-700 rounded text-zinc-200 text-[10px] focus:outline-none focus:border-zinc-500"
          />
        </label>
      </div>

      <button
        data-testid="btn-apply-clip"
        onClick={handleApply}
        className="w-full px-2 py-1 text-[10px] bg-blue-600 hover:bg-blue-500 rounded text-white"
      >
        应用
      </button>
    </div>
  );
}
