import { useEffect, useState, useRef, useCallback } from "react";
import { Toolbar } from "./components/Toolbar";
import { AssetLibrary } from "./components/AssetLibrary";
import { MetadataView } from "./components/MetadataView";
import { TaskPanel } from "./components/TaskPanel";
import { PreviewPlayer } from "./components/PreviewPlayer";
import { TimelineView } from "./components/TimelineView";
import { MarkerPanel } from "./components/MarkerPanel";
import { ClipInfoPanel } from "./components/ClipInfoPanel";
import { useProjectStore, initEventSubscriptions } from "./store/projectStore";

type RightTab = "tasks" | "markers" | "clip" | "detail";

function ProjectInfo() {
  const { projectFile, projectDir } = useProjectStore();
  if (!projectFile) return null;
  const { project } = projectFile;

  return (
    <div
      data-testid="project-info"
      className="px-4 py-1.5 bg-zinc-900/50 border-b border-zinc-800 text-xs text-zinc-500 flex gap-4"
    >
      <span>
        分辨率: {project.settings.resolution.width}x
        {project.settings.resolution.height}
      </span>
      <span>帧率: {project.settings.fps} fps</span>
      <span>采样率: {project.settings.sampleRate} Hz</span>
      <span className="ml-auto">{projectDir}</span>
    </div>
  );
}

function ErrorBanner() {
  const { error, clearError } = useProjectStore();
  if (!error) return null;

  return (
    <div
      data-testid="error-banner"
      className="px-4 py-2 bg-red-900/50 border-b border-red-800 text-xs text-red-300 flex items-center justify-between"
    >
      <span>错误: {error}</span>
      <button
        onClick={clearError}
        className="px-2 py-0.5 text-xs bg-red-800 hover:bg-red-700 rounded"
      >
        关闭
      </button>
    </div>
  );
}

function ResizeHandle({
  onResize,
}: {
  onResize: (deltaY: number) => void;
}) {
  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      e.preventDefault();
      const startY = e.clientY;

      const onMove = (ev: PointerEvent) => {
        onResize(ev.clientY - startY);
      };

      const onUp = () => {
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);
      };

      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
    },
    [onResize]
  );

  return (
    <div
      className="h-1.5 cursor-row-resize bg-zinc-800 hover:bg-zinc-600 transition-colors flex items-center justify-center"
      onPointerDown={handlePointerDown}
    >
      <div className="w-8 h-0.5 bg-zinc-600 rounded-full" />
    </div>
  );
}

export default function App() {
  const [rightTab, setRightTab] = useState<RightTab>("tasks");
  const [timelineHeight, setTimelineHeight] = useState(240);
  const baseHeightRef = useRef(240);
  const selectedAssetId = useProjectStore((s) => s.selectedAssetId);

  useEffect(() => {
    initEventSubscriptions();
  }, []);

  useEffect(() => {
    if (selectedAssetId) {
      setRightTab("detail");
    }
  }, [selectedAssetId]);

  const handleResize = useCallback((deltaY: number) => {
    setTimelineHeight(Math.max(120, Math.min(500, baseHeightRef.current - deltaY)));
  }, []);

  useEffect(() => {
    baseHeightRef.current = timelineHeight;
  }, [timelineHeight]);

  return (
    <div className="flex flex-col h-screen bg-zinc-950 text-zinc-100">
      <Toolbar />
      <ErrorBanner />
      <ProjectInfo />

      {/* Upper panel */}
      <div className="flex flex-1 overflow-hidden min-h-0">
        {/* Left: Asset Library */}
        <div className="w-72 border-r border-zinc-800 flex flex-col overflow-hidden">
          <AssetLibrary />
        </div>

        {/* Center: Preview */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <PreviewPlayer />
        </div>

        {/* Right: Tab panel */}
        <div className="w-72 border-l border-zinc-800 flex flex-col overflow-hidden">
          {/* Tab bar */}
          <div className="flex border-b border-zinc-800 bg-zinc-900/50">
            {(
              [
                { key: "tasks", label: "任务" },
                { key: "markers", label: "标记" },
                { key: "clip", label: "Clip" },
                { key: "detail", label: "详情" },
              ] as { key: RightTab; label: string }[]
            ).map(({ key, label }) => (
              <button
                key={key}
                data-testid={`tab-${key}`}
                onClick={() => setRightTab(key)}
                className={`flex-1 px-2 py-1.5 text-[10px] font-medium transition-colors ${
                  rightTab === key
                    ? "text-zinc-100 border-b-2 border-blue-500"
                    : "text-zinc-500 hover:text-zinc-300"
                }`}
              >
                {label}
              </button>
            ))}
          </div>

          {/* Tab content */}
          <div className="flex-1 overflow-hidden">
            {rightTab === "tasks" && <TaskPanel />}
            {rightTab === "markers" && <MarkerPanel />}
            {rightTab === "clip" && <ClipInfoPanel />}
            {rightTab === "detail" && <MetadataView />}
          </div>
        </div>
      </div>

      {/* Resize handle */}
      <ResizeHandle onResize={handleResize} />

      {/* Lower panel: Timeline */}
      <div
        className="border-t border-zinc-800 overflow-hidden"
        style={{ height: timelineHeight }}
      >
        <TimelineView />
      </div>
    </div>
  );
}
