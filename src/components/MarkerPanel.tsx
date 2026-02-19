import { useState, useCallback } from "react";
import { useProjectStore } from "../store/projectStore";
import { useTimelineViewStore, formatMs } from "../store/timelineViewStore";
import * as commands from "../services/commands";
import type { Marker } from "../models/project";

function MarkerRow({
  marker,
  selected,
  onSelect,
  onJump,
  onDelete,
}: {
  marker: Marker;
  selected: boolean;
  onSelect: () => void;
  onJump: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      className={`px-3 py-2 border-b border-zinc-800 cursor-pointer text-sm ${
        selected
          ? "bg-yellow-900/20 border-l-2 border-l-yellow-500"
          : "hover:bg-zinc-800/50"
      }`}
      onClick={onSelect}
    >
      <div className="flex items-center gap-2">
        <span
          className="text-[10px] font-mono text-yellow-400 cursor-pointer hover:underline shrink-0"
          onClick={(e) => {
            e.stopPropagation();
            onJump();
          }}
        >
          {formatMs(marker.tMs)}
        </span>
        <span className="text-xs text-zinc-300 truncate flex-1">
          {marker.label || "(无标签)"}
        </span>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="text-[10px] text-zinc-500 hover:text-red-400 shrink-0"
          title="删除"
        >
          ✕
        </button>
      </div>
      {marker.promptText && (
        <div className="text-[10px] text-zinc-500 mt-0.5 line-clamp-2">
          {marker.promptText}
        </div>
      )}
    </div>
  );
}

export function MarkerPanel() {
  const { projectFile } = useProjectStore();
  const { playheadMs, setPlayhead } = useTimelineViewStore();
  const [selectedMarkerId, setSelectedMarkerId] = useState<string | null>(null);
  const [editLabel, setEditLabel] = useState("");
  const [editPrompt, setEditPrompt] = useState("");

  const markers = projectFile?.timeline?.markers ?? [];

  const selectedMarker = markers.find((m) => m.markerId === selectedMarkerId);

  const handleSelect = useCallback(
    (marker: Marker) => {
      setSelectedMarkerId(marker.markerId);
      setEditLabel(marker.label);
      setEditPrompt(marker.promptText);
    },
    []
  );

  const handleAddMarker = useCallback(async () => {
    try {
      const m = await commands.markerAdd(Math.round(playheadMs), "标记", "");
      setSelectedMarkerId(m.markerId);
      setEditLabel(m.label);
      setEditPrompt(m.promptText);
    } catch (err) {
      console.error("Failed to add marker:", err);
    }
  }, [playheadMs]);

  const handleSave = useCallback(async () => {
    if (!selectedMarkerId) return;
    try {
      await commands.markerUpdate(
        selectedMarkerId,
        editLabel || undefined,
        editPrompt || undefined
      );
    } catch (err) {
      console.error("Failed to update marker:", err);
    }
  }, [selectedMarkerId, editLabel, editPrompt]);

  const handleDelete = useCallback(
    async (markerId: string) => {
      try {
        await commands.markerRemove(markerId);
        if (selectedMarkerId === markerId) {
          setSelectedMarkerId(null);
        }
      } catch (err) {
        console.error("Failed to remove marker:", err);
      }
    },
    [selectedMarkerId]
  );

  if (!projectFile) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        打开项目以管理标记
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="px-3 py-2 text-xs text-zinc-400 font-semibold border-b border-zinc-800 flex items-center justify-between">
        <span>标记 ({markers.length})</span>
        <button
          onClick={handleAddMarker}
          className="px-2 py-0.5 text-[10px] bg-yellow-600 hover:bg-yellow-500 rounded text-white"
        >
          + 添加标记
        </button>
      </div>

      {/* Marker list */}
      <div className="flex-1 overflow-y-auto">
        {markers.length === 0 ? (
          <div className="flex items-center justify-center h-20 text-zinc-500 text-xs">
            暂无标记
          </div>
        ) : (
          markers.map((m) => (
            <MarkerRow
              key={m.markerId}
              marker={m}
              selected={selectedMarkerId === m.markerId}
              onSelect={() => handleSelect(m)}
              onJump={() => setPlayhead(m.tMs)}
              onDelete={() => handleDelete(m.markerId)}
            />
          ))
        )}
      </div>

      {/* Edit panel */}
      {selectedMarker && (
        <div className="border-t border-zinc-800 px-3 py-2 space-y-2 bg-zinc-900/50">
          <div className="text-[10px] text-zinc-400 font-semibold">
            编辑标记 · {formatMs(selectedMarker.tMs)}
          </div>
          <input
            type="text"
            value={editLabel}
            onChange={(e) => setEditLabel(e.target.value)}
            placeholder="标签名称"
            className="w-full px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200 focus:outline-none focus:border-zinc-500"
          />
          <textarea
            value={editPrompt}
            onChange={(e) => setEditPrompt(e.target.value)}
            placeholder="Prompt 文本..."
            rows={4}
            className="w-full px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200 focus:outline-none focus:border-zinc-500 resize-none"
          />
          <button
            onClick={handleSave}
            className="w-full px-2 py-1 text-[10px] bg-blue-600 hover:bg-blue-500 rounded text-white"
          >
            保存
          </button>
        </div>
      )}
    </div>
  );
}
