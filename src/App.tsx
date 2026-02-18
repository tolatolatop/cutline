import { useEffect } from "react";
import { Toolbar } from "./components/Toolbar";
import { AssetLibrary } from "./components/AssetLibrary";
import { MetadataView } from "./components/MetadataView";
import { TaskPanel } from "./components/TaskPanel";
import { useProjectStore, initEventSubscriptions } from "./store/projectStore";

function ProjectInfo() {
  const { projectFile, projectDir } = useProjectStore();
  if (!projectFile) return null;
  const { project } = projectFile;

  return (
    <div className="px-4 py-2 bg-zinc-900/50 border-b border-zinc-800 text-xs text-zinc-500 flex gap-4">
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
    <div className="px-4 py-2 bg-red-900/50 border-b border-red-800 text-xs text-red-300 flex items-center justify-between">
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

export default function App() {
  useEffect(() => {
    initEventSubscriptions();
  }, []);

  return (
    <div className="flex flex-col h-screen bg-zinc-950 text-zinc-100">
      <Toolbar />
      <ErrorBanner />
      <ProjectInfo />
      <div className="flex flex-1 overflow-hidden">
        {/* Left: Asset Library */}
        <div className="w-80 border-r border-zinc-800 flex flex-col">
          <AssetLibrary />
        </div>

        {/* Center: Metadata */}
        <div className="flex-1 flex flex-col">
          <MetadataView />
        </div>

        {/* Right: Task Panel */}
        <div className="w-80 border-l border-zinc-800 flex flex-col">
          <TaskPanel />
        </div>
      </div>
    </div>
  );
}
