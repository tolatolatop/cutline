import { useState, useEffect } from "react";
import { useProjectStore } from "../store/projectStore";
import { useProviderStore } from "../store/providerStore";
import * as commands from "../services/commands";

const VIDEO_MODELS = [
  { value: "jimeng-video-3.0", label: "即梦视频 3.0" },
  { value: "jimeng-video-3.0-pro", label: "即梦视频 3.0 Pro" },
  { value: "jimeng-video-2.0-pro", label: "即梦视频 2.0 Pro" },
  { value: "jimeng-video-2.0", label: "即梦视频 2.0" },
  { value: "seedance-2.0", label: "Seedance 2.0" },
];

const RATIOS = [
  { value: "16:9", label: "16:9 横屏" },
  { value: "9:16", label: "9:16 竖屏" },
  { value: "1:1", label: "1:1 方形" },
];

export function GeneratePanel() {
  const { projectFile } = useProjectStore();
  const { providers, loadProviders } = useProviderStore();

  const [prompt, setPrompt] = useState("");
  const [model, setModel] = useState("jimeng-video-3.0");
  const [ratio, setRatio] = useState("16:9");
  const [durationSec, setDurationSec] = useState(5);
  const [providerName, setProviderName] = useState("jimeng");
  const [profileName, setProfileName] = useState("default");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<{ taskId: string } | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  const genVideoTask = projectFile?.tasks?.find(
    (t) => t.kind === "gen_video" && (t.state === "running" || t.state === "queued")
  );

  const handleGenerate = async () => {
    if (!prompt.trim()) return;
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await commands.genVideoEnqueue({
        providerName,
        profileName,
        prompt: prompt.trim(),
        model,
        ratio,
        durationMs: durationSec * 1000,
      });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleExport = async () => {
    setLoading(true);
    setError(null);
    try {
      await commands.exportDraft();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  if (!projectFile) {
    return (
      <div className="flex items-center justify-center h-full text-zinc-600 text-xs">
        请先打开项目
      </div>
    );
  }

  const draftTrack = projectFile.timeline.tracks.find(
    (t) => t.trackId === "trk_draft"
  );
  const draftClipCount = draftTrack?.clipIds.length ?? 0;

  return (
    <div data-testid="generate-panel" className="flex flex-col h-full">
      <div className="px-3 py-2 text-xs text-zinc-400 font-semibold border-b border-zinc-800">
        视频生成
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {/* Prompt */}
        <div>
          <label className="block text-[10px] text-zinc-500 mb-1">提示词</label>
          <textarea
            data-testid="gen-prompt"
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="描述你想生成的视频内容..."
            rows={3}
            className="w-full bg-zinc-800 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 resize-none focus:outline-none focus:border-blue-500"
          />
        </div>

        {/* Model */}
        <div>
          <label className="block text-[10px] text-zinc-500 mb-1">模型</label>
          <select
            data-testid="gen-model"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            className="w-full bg-zinc-800 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-blue-500"
          >
            {VIDEO_MODELS.map((m) => (
              <option key={m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </select>
        </div>

        {/* Ratio */}
        <div>
          <label className="block text-[10px] text-zinc-500 mb-1">比例</label>
          <div className="flex gap-1">
            {RATIOS.map((r) => (
              <button
                key={r.value}
                onClick={() => setRatio(r.value)}
                className={`flex-1 px-2 py-1 text-[10px] rounded border ${
                  ratio === r.value
                    ? "bg-blue-600 border-blue-500 text-white"
                    : "bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-zinc-200"
                }`}
              >
                {r.label}
              </button>
            ))}
          </div>
        </div>

        {/* Duration */}
        <div>
          <label className="block text-[10px] text-zinc-500 mb-1">
            时长: {durationSec}s
          </label>
          <input
            data-testid="gen-duration"
            type="range"
            min={3}
            max={15}
            step={1}
            value={durationSec}
            onChange={(e) => setDurationSec(Number(e.target.value))}
            className="w-full accent-blue-500"
          />
          <div className="flex justify-between text-[9px] text-zinc-600">
            <span>3s</span>
            <span>15s</span>
          </div>
        </div>

        {/* Provider / Profile */}
        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className="block text-[10px] text-zinc-500 mb-1">Provider</label>
            <select
              data-testid="gen-provider"
              value={providerName}
              onChange={(e) => setProviderName(e.target.value)}
              className="w-full bg-zinc-800 border border-zinc-700 rounded px-2 py-1 text-[10px] text-zinc-200 focus:outline-none focus:border-blue-500"
            >
              {providers.length > 0 ? (
                providers.map((p) => (
                  <option key={p.name} value={p.name}>
                    {p.displayName}
                  </option>
                ))
              ) : (
                <option value="jimeng">即梦</option>
              )}
            </select>
          </div>
          <div>
            <label className="block text-[10px] text-zinc-500 mb-1">Profile</label>
            <input
              data-testid="gen-profile"
              type="text"
              value={profileName}
              onChange={(e) => setProfileName(e.target.value)}
              className="w-full bg-zinc-800 border border-zinc-700 rounded px-2 py-1 text-[10px] text-zinc-200 focus:outline-none focus:border-blue-500"
            />
          </div>
        </div>

        {/* Generate Button */}
        <button
          data-testid="gen-submit"
          onClick={handleGenerate}
          disabled={loading || !prompt.trim() || !!genVideoTask}
          className="w-full py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 disabled:text-zinc-500 text-white text-xs font-medium rounded transition-colors"
        >
          {genVideoTask
            ? `生成中... ${genVideoTask.progress?.percent?.toFixed(0) ?? 0}%`
            : loading
            ? "提交中..."
            : "生成视频"}
        </button>

        {/* Active gen_video task progress */}
        {genVideoTask && genVideoTask.progress && (
          <div className="space-y-1">
            <div className="w-full bg-zinc-700 rounded-full h-1.5">
              <div
                className="bg-blue-500 h-1.5 rounded-full transition-all duration-500"
                style={{
                  width: `${Math.min(100, genVideoTask.progress.percent ?? 0)}%`,
                }}
              />
            </div>
            <div className="text-[10px] text-zinc-500">
              {genVideoTask.progress.phase}
              {genVideoTask.progress.message && `: ${genVideoTask.progress.message}`}
            </div>
          </div>
        )}

        {/* Result / Error */}
        {result && (
          <div className="p-2 bg-green-900/30 border border-green-800 rounded text-[10px] text-green-300">
            任务已提交: {result.taskId}
          </div>
        )}

        {error && (
          <div className="p-2 bg-red-900/30 border border-red-800 rounded text-[10px] text-red-300">
            {error}
          </div>
        )}

        {/* Draft track info + export */}
        <div className="border-t border-zinc-800 pt-3 mt-2">
          <div className="text-[10px] text-zinc-500 mb-2">
            草稿轨道: {draftClipCount} 个片段
          </div>
          <button
            data-testid="gen-export"
            onClick={handleExport}
            disabled={loading || draftClipCount === 0}
            className="w-full py-1.5 bg-zinc-700 hover:bg-zinc-600 disabled:bg-zinc-800 disabled:text-zinc-600 text-zinc-200 text-xs rounded transition-colors"
          >
            导出草稿轨道
          </button>
        </div>
      </div>
    </div>
  );
}
