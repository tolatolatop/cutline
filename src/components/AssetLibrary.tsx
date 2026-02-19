import { useState, useMemo } from "react";
import { useProjectStore } from "../store/projectStore";
import { useThumbnail } from "../hooks/useThumbnail";
import type { Asset, VideoMeta, AudioMeta } from "../models/project";

type ViewMode = "grid" | "list";
type FilterType = "all" | "video" | "audio" | "image";

function fileName(path: string) {
  return path.split(/[/\\]/).pop() || path;
}

function formatDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function getThumbRelativePath(asset: Asset): string | null {
  const meta = asset.meta as unknown as Record<string, unknown>;
  return (meta?.thumbUri as string) ?? null;
}

function getDuration(asset: Asset): number | null {
  const meta = asset.meta as unknown as Record<string, unknown>;
  if (meta?.kind === "video") return (meta as unknown as VideoMeta).durationSec;
  if (meta?.kind === "audio") return (meta as unknown as AudioMeta).durationSec;
  return null;
}

function getResolution(asset: Asset): string | null {
  const meta = asset.meta as unknown as Record<string, unknown>;
  if (meta?.kind === "video") {
    const vm = meta as unknown as VideoMeta;
    return `${vm.width}x${vm.height}`;
  }
  return null;
}

function assetIcon(type: string) {
  switch (type) {
    case "video": return "🎬";
    case "audio": return "🎵";
    case "image": return "🖼️";
    case "prompt": return "📝";
    default: return "📄";
  }
}

function ThumbnailImg({
  relativePath,
  alt,
  className,
}: {
  relativePath: string | null;
  alt?: string;
  className?: string;
}) {
  const dataUrl = useThumbnail(relativePath);
  if (!dataUrl) return null;
  return <img src={dataUrl} alt={alt ?? ""} className={className} />;
}

function AssetGridCard({
  asset,
  selected,
  onSelect,
  testIndex,
}: {
  asset: Asset;
  selected: boolean;
  onSelect: () => void;
  testIndex: number;
}) {
  const thumbPath = getThumbRelativePath(asset);
  const duration = getDuration(asset);
  const resolution = getResolution(asset);

  return (
    <div
      data-testid={`asset-card-${testIndex}`}
      onClick={onSelect}
      className={`relative cursor-pointer rounded overflow-hidden border-2 transition-colors ${
        selected ? "border-blue-500" : "border-zinc-800 hover:border-zinc-600"
      }`}
    >
      <div className="aspect-video bg-zinc-800 flex items-center justify-center">
        {thumbPath ? (
          <ThumbnailImg
            relativePath={thumbPath}
            alt={fileName(asset.path)}
            className="w-full h-full object-cover"
          />
        ) : (
          <span className="text-2xl">{assetIcon(asset.type)}</span>
        )}
      </div>

      {duration != null && (
        <div className="absolute bottom-7 right-1 px-1 py-0.5 bg-black/70 rounded text-[10px] text-white font-mono">
          {formatDuration(duration)}
        </div>
      )}

      {resolution && (
        <div className="absolute top-1 right-1 px-1 py-0.5 bg-black/70 rounded text-[10px] text-white font-mono">
          {resolution}
        </div>
      )}

      <div className="px-1.5 py-1 bg-zinc-900">
        <div className="text-[11px] text-zinc-300 truncate">
          {fileName(asset.path)}
        </div>
      </div>
    </div>
  );
}

function AssetListRow({
  asset,
  selected,
  onSelect,
  testIndex,
}: {
  asset: Asset;
  selected: boolean;
  onSelect: () => void;
  testIndex: number;
}) {
  const thumbPath = getThumbRelativePath(asset);
  const duration = getDuration(asset);

  return (
    <div
      data-testid={`asset-card-${testIndex}`}
      onClick={onSelect}
      className={`flex items-center gap-2 px-3 py-2 cursor-pointer border-b border-zinc-800 text-sm ${
        selected
          ? "bg-blue-900/40 border-l-2 border-l-blue-500"
          : "hover:bg-zinc-800/50"
      }`}
    >
      <div className="w-10 h-10 rounded bg-zinc-800 flex items-center justify-center overflow-hidden shrink-0">
        {thumbPath ? (
          <ThumbnailImg
            relativePath={thumbPath}
            className="w-full h-full object-cover"
          />
        ) : (
          <span className="text-base">{assetIcon(asset.type)}</span>
        )}
      </div>
      <div className="flex-1 min-w-0">
        <div className="text-zinc-200 truncate text-xs">
          {fileName(asset.path)}
        </div>
        <div className="text-[10px] text-zinc-500 truncate">
          {asset.type}
          {duration != null && ` · ${formatDuration(duration)}`}
        </div>
      </div>
    </div>
  );
}

export function AssetLibrary() {
  const { projectFile, selectedAssetId, selectAsset } =
    useProjectStore();
  const [viewMode, setViewMode] = useState<ViewMode>("grid");
  const [filterType, setFilterType] = useState<FilterType>("all");

  const filtered = useMemo(() => {
    if (!projectFile) return [];
    if (filterType === "all") return projectFile.assets;
    return projectFile.assets.filter((a) => a.type === filterType);
  }, [projectFile, filterType]);

  if (!projectFile) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        新建或打开一个项目以开始
      </div>
    );
  }

  return (
    <div data-testid="asset-library" className="flex-1 flex flex-col overflow-hidden">
      {/* Header */}
      <div className="px-3 py-2 text-xs text-zinc-400 font-semibold border-b border-zinc-800 flex items-center justify-between">
        <span>素材库 ({filtered.length})</span>
        <div className="flex gap-1">
          <button
            onClick={() => setViewMode("grid")}
            className={`px-1.5 py-0.5 text-[10px] rounded ${
              viewMode === "grid"
                ? "bg-zinc-600 text-zinc-100"
                : "text-zinc-500 hover:text-zinc-300"
            }`}
            title="网格"
          >
            ▦
          </button>
          <button
            onClick={() => setViewMode("list")}
            className={`px-1.5 py-0.5 text-[10px] rounded ${
              viewMode === "list"
                ? "bg-zinc-600 text-zinc-100"
                : "text-zinc-500 hover:text-zinc-300"
            }`}
            title="列表"
          >
            ☰
          </button>
        </div>
      </div>

      {/* Filter bar */}
      <div className="px-3 py-1 flex gap-1 border-b border-zinc-800">
        {(["all", "video", "audio", "image"] as FilterType[]).map((f) => {
          const count =
            f === "all"
              ? projectFile.assets.length
              : projectFile.assets.filter((a) => a.type === f).length;
          return (
            <button
              key={f}
              data-testid={`asset-filter-${f}`}
              onClick={() => setFilterType(f)}
              className={`px-2 py-0.5 text-[10px] rounded ${
                filterType === f
                  ? "bg-zinc-600 text-zinc-100"
                  : "text-zinc-500 hover:text-zinc-300"
              }`}
            >
              {f === "all" ? "全部" : f} ({count})
            </button>
          );
        })}
      </div>

      {/* Content */}
      {filtered.length === 0 ? (
        <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
          {projectFile.assets.length === 0
            ? "暂无素材，点击「导入素材」添加"
            : "无匹配素材"}
        </div>
      ) : viewMode === "grid" ? (
        <div className="flex-1 overflow-y-auto p-2">
          <div className="grid grid-cols-2 gap-2">
            {filtered.map((asset, idx) => (
              <AssetGridCard
                key={asset.assetId}
                asset={asset}
                selected={selectedAssetId === asset.assetId}
                onSelect={() => selectAsset(asset.assetId)}
                testIndex={idx}
              />
            ))}
          </div>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto">
          {filtered.map((asset, idx) => (
            <AssetListRow
              key={asset.assetId}
              asset={asset}
              selected={selectedAssetId === asset.assetId}
              onSelect={() => selectAsset(asset.assetId)}
              testIndex={idx}
            />
          ))}
        </div>
      )}
    </div>
  );
}
