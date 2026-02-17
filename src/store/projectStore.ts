import { create } from "zustand";
import type { ProjectFile } from "../models/project";
import * as commands from "../services/commands";

interface ProjectState {
  projectFile: ProjectFile | null;
  projectDir: string | null;
  projectJsonPath: string | null;
  selectedAssetId: string | null;
  isDirty: boolean;
  loading: boolean;
  error: string | null;

  createProject: (dirPath: string, name: string) => Promise<void>;
  openProject: (projectJsonPath: string) => Promise<void>;
  saveProject: () => Promise<void>;
  importAssets: (filePaths: string[]) => Promise<void>;
  selectAsset: (assetId: string | null) => void;
  clearError: () => void;
}

function rebuildIndexes(pf: ProjectFile): ProjectFile {
  const assetById: Record<string, number> = {};
  const taskById: Record<string, number> = {};
  pf.assets.forEach((a, i) => {
    assetById[a.assetId] = i;
  });
  pf.tasks.forEach((t, i) => {
    taskById[t.taskId] = i;
  });
  return { ...pf, indexes: { assetById, taskById } };
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projectFile: null,
  projectDir: null,
  projectJsonPath: null,
  selectedAssetId: null,
  isDirty: false,
  loading: false,
  error: null,

  createProject: async (dirPath, name) => {
    set({ loading: true, error: null });
    try {
      const pf = await commands.createProject(dirPath, name);
      const projectJsonPath = `${dirPath}\\project.json`;
      set({
        projectFile: pf,
        projectDir: dirPath,
        projectJsonPath,
        isDirty: false,
        loading: false,
        selectedAssetId: null,
      });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  openProject: async (projectJsonPath) => {
    set({ loading: true, error: null });
    try {
      const pf = await commands.openProject(projectJsonPath);
      const parts = projectJsonPath.replace(/\//g, "\\").split("\\");
      parts.pop();
      const projectDir = parts.join("\\");
      set({
        projectFile: pf,
        projectDir,
        projectJsonPath,
        isDirty: false,
        loading: false,
        selectedAssetId: null,
      });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  saveProject: async () => {
    const { projectFile, projectJsonPath } = get();
    if (!projectFile || !projectJsonPath) return;
    set({ loading: true, error: null });
    try {
      const updated = rebuildIndexes({
        ...projectFile,
        project: {
          ...projectFile.project,
          updatedAt: new Date().toISOString(),
        },
      });
      await commands.saveProject(projectJsonPath, updated);
      set({ projectFile: updated, isDirty: false, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  importAssets: async (filePaths) => {
    const { projectFile, projectDir } = get();
    if (!projectFile || !projectDir) return;
    set({ loading: true, error: null });
    try {
      const newAssets = await commands.importAssets(projectDir, filePaths);
      const allAssets = [...projectFile.assets, ...newAssets];
      const updated = rebuildIndexes({
        ...projectFile,
        assets: allAssets,
      });
      set({ projectFile: updated, isDirty: true, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  selectAsset: (assetId) => set({ selectedAssetId: assetId }),
  clearError: () => set({ error: null }),
}));
