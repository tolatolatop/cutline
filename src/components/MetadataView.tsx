import { useProjectStore } from "../store/projectStore";
import type { Asset, VideoMeta, AudioMeta, ImageMeta } from "../models/project";

function MetaField({ label, value }: { label: string; value: string | number | undefined }) {
  if (value === undefined) return null;
  return (
    <div className="flex justify-between py-1 border-b border-zinc-800">
      <span className="text-zinc-500 text-xs">{label}</span>
      <span className="text-zinc-300 text-xs font-mono">{String(value)}</span>
    </div>
  );
}

function VideoMetaView({ meta }: { meta: VideoMeta }) {
  return (
    <>
      <MetaField label="容器" value={meta.container} />
      <MetaField label="编码" value={meta.codec} />
      <MetaField label="时长" value={`${meta.durationSec.toFixed(2)}s`} />
      <MetaField label="分辨率" value={`${meta.width}×${meta.height}`} />
      <MetaField label="帧率" value={`${meta.fps} fps`} />
      {meta.audio && (
        <>
          <MetaField label="音频" value={meta.audio.present ? "有" : "无"} />
          <MetaField label="采样率" value={`${meta.audio.sampleRate} Hz`} />
          <MetaField label="声道" value={meta.audio.channels} />
        </>
      )}
    </>
  );
}

function AudioMetaView({ meta }: { meta: AudioMeta }) {
  return (
    <>
      <MetaField label="编码" value={meta.codec} />
      <MetaField label="时长" value={`${meta.durationSec.toFixed(2)}s`} />
      <MetaField label="采样率" value={`${meta.sampleRate} Hz`} />
      <MetaField label="声道" value={meta.channels} />
    </>
  );
}

function ImageMetaView({ meta }: { meta: ImageMeta }) {
  return (
    <>
      <MetaField label="格式" value={meta.format} />
      <MetaField label="尺寸" value={`${meta.width}×${meta.height}`} />
    </>
  );
}

function AssetDetail({ asset }: { asset: Asset }) {
  const meta = asset.meta;

  return (
    <div className="p-3 space-y-3">
      <div>
        <h3 className="text-sm font-semibold text-zinc-200 mb-1">
          {asset.path.split(/[/\\]/).pop()}
        </h3>
        <div className="text-xs text-zinc-500">
          {asset.type} · {asset.source}
        </div>
      </div>

      <div>
        <div className="text-xs text-zinc-400 font-semibold mb-1">指纹</div>
        <div className="text-[11px] font-mono text-zinc-500 break-all bg-zinc-800/50 p-2 rounded">
          {asset.fingerprint.value}
        </div>
        <div className="text-[10px] text-zinc-600 mt-1">
          算法: {asset.fingerprint.algo} · 基础: {asset.fingerprint.basis}
        </div>
      </div>

      <div>
        <div className="text-xs text-zinc-400 font-semibold mb-1">元数据</div>
        {"kind" in meta && meta.kind === "video" && <VideoMetaView meta={meta as VideoMeta} />}
        {"kind" in meta && meta.kind === "audio" && <AudioMetaView meta={meta as AudioMeta} />}
        {"kind" in meta && meta.kind === "image" && <ImageMetaView meta={meta as ImageMeta} />}
      </div>

      {asset.tags.length > 0 && (
        <div>
          <div className="text-xs text-zinc-400 font-semibold mb-1">标签</div>
          <div className="flex flex-wrap gap-1">
            {asset.tags.map((tag) => (
              <span key={tag} className="px-2 py-0.5 text-[10px] bg-zinc-700 text-zinc-400 rounded">
                {tag}
              </span>
            ))}
          </div>
        </div>
      )}

      <div>
        <div className="text-xs text-zinc-400 font-semibold mb-1">路径</div>
        <div className="text-[11px] font-mono text-zinc-500 break-all">
          {asset.path}
        </div>
      </div>

      <div>
        <MetaField label="创建时间" value={new Date(asset.createdAt).toLocaleString()} />
        <MetaField label="Asset ID" value={asset.assetId} />
      </div>
    </div>
  );
}

export function MetadataView() {
  const { projectFile, selectedAssetId } = useProjectStore();

  if (!projectFile || !selectedAssetId) {
    return (
      <div data-testid="metadata-view" className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        选择一个素材查看详情
      </div>
    );
  }

  const asset = projectFile.assets.find((a) => a.assetId === selectedAssetId);
  if (!asset) {
    return (
      <div data-testid="metadata-view" className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
        素材不存在
      </div>
    );
  }

  return (
    <div data-testid="metadata-view" className="flex-1 overflow-y-auto">
      <div className="px-3 py-2 text-xs text-zinc-400 font-semibold border-b border-zinc-800">
        素材详情
      </div>
      <AssetDetail asset={asset} />
    </div>
  );
}
