import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../store/projectStore";

export function Toolbar() {
  const {
    projectFile,
    isDirty,
    loading,
    createProject,
    openProject,
    saveProject,
    importAssets,
  } = useProjectStore();

  const handleNew = async () => {
    const dir = await open({ directory: true, title: "选择项目目录" });
    if (!dir) return;
    const name = prompt("项目名称", "Untitled Project");
    if (!name) return;
    await createProject(dir as string, name);
  };

  const handleOpen = async () => {
    const file = await open({
      title: "打开项目",
      filters: [{ name: "Project", extensions: ["json"] }],
    });
    if (!file) return;
    await openProject(file as string);
  };

  const handleSave = async () => {
    if (!projectFile) return;
    await saveProject();
  };

  const handleImport = async () => {
    if (!projectFile) return;
    const files = await open({
      title: "导入素材",
      multiple: true,
      filters: [
        {
          name: "媒体文件",
          extensions: [
            "mp4", "mov", "avi", "mkv", "webm",
            "mp3", "wav", "aac", "flac",
            "png", "jpg", "jpeg", "webp", "bmp",
          ],
        },
      ],
    });
    if (!files) return;
    const paths = Array.isArray(files) ? files : [files];
    await importAssets(paths as string[]);
  };

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-zinc-900 border-b border-zinc-700">
      <span className="text-sm font-bold text-zinc-300 mr-4">Cutline</span>
      <button
        onClick={handleNew}
        disabled={loading}
        className="px-3 py-1.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded disabled:opacity-50"
      >
        新建项目
      </button>
      <button
        onClick={handleOpen}
        disabled={loading}
        className="px-3 py-1.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded disabled:opacity-50"
      >
        打开项目
      </button>
      <button
        onClick={handleSave}
        disabled={loading || !projectFile}
        className="px-3 py-1.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded disabled:opacity-50"
      >
        保存{isDirty ? " *" : ""}
      </button>
      <button
        onClick={handleImport}
        disabled={loading || !projectFile}
        className="px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded disabled:opacity-50"
      >
        导入素材
      </button>
      {projectFile && (
        <span className="ml-auto text-xs text-zinc-500">
          {projectFile.project.name} — {projectFile.assets.length} 个素材
        </span>
      )}
    </div>
  );
}
