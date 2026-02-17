import { useProjectStore } from "../store/projectStore";
import type { Asset } from "../models/project";

function assetIcon(type: string) {
  switch (type) {
    case "video": return "🎬";
    case "audio": return "🎵";
    case "image": return "🖼️";
    case "prompt": return "📝";
    default: return "📄";
  }
}

function fileName(path: string) {
  return path.split(/[/\\]/).pop() || path;
}

function AssetRow({ asset, selected, onSelect }: {
  asset: Asset;
  selected: boolean;
  onSelect: () => void;
}) {
  const fingerShort = asset.fingerprint.value.slice(0, 20) + "…";

  return (
    <div
      onClick={onSelect}
      className={`flex items-center gap-2 px-3 py-2 cursor-pointer border-b border-zinc-800 text-sm
        ${selected ? "bg-blue-900/40 border-l-2 border-l-blue-500" : "hover:bg-zinc-800/50"}`}
    >
      <span className="text-base">{assetIcon(asset.type)}</span>
      <div className="flex-1 min-w-0">
        <div className="text-zinc-200 truncate">{fileName(asset.path)}</div>
        <div className="text-xs text-zinc-500 truncate">
          {asset.type} · {asset.source} · {fingerShort}
        </div>
      </div>
      {asset.tags.length > 0 && (
        <div className="flex gap-1">
          {asset.tags.slice(0, 2).map((tag) => (
            <span key={tag} className="px-1.5 py-0.5 text-[10px] bg-zinc-700 text-zinc-400 rounded">
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

export function AssetLibrary() {
  const { projectFile, selectedAssetId, selectAsset } = useProjectStore();

  if (!projectFile) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        新建或打开一个项目以开始
      </div>
    );
  }

  if (projectFile.assets.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        暂无素材，点击「导入素材」添加
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto">
      <div className="px-3 py-2 text-xs text-zinc-400 font-semibold border-b border-zinc-800">
        素材库 ({projectFile.assets.length})
      </div>
      {projectFile.assets.map((asset) => (
        <AssetRow
          key={asset.assetId}
          asset={asset}
          selected={selectedAssetId === asset.assetId}
          onSelect={() => selectAsset(asset.assetId)}
        />
      ))}
    </div>
  );
}
