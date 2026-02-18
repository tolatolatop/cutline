import { create } from "zustand";
import type { ProjectFile, Task } from "../models/project";
import * as commands from "../services/commands";
import { subscribeTaskUpdates, subscribeProjectUpdates } from "../services/events";

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
  refreshProject: () => Promise<void>;
  selectAsset: (assetId: string | null) => void;
  clearError: () => void;
  updateTask: (task: Task) => void;
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
    const { projectFile } = get();
    if (!projectFile) return;
    set({ loading: true, error: null });
    try {
      await commands.saveProject();
      const fresh = await commands.getProject();
      set({ projectFile: fresh, isDirty: false, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  importAssets: async (filePaths) => {
    const { projectFile } = get();
    if (!projectFile) return;
    set({ loading: true, error: null });
    try {
      await commands.importAssets(filePaths);
      const fresh = await commands.getProject();
      set({ projectFile: fresh, isDirty: false, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  refreshProject: async () => {
    try {
      const fresh = await commands.getProject();
      set({ projectFile: fresh });
    } catch {
      // Project might not be loaded yet
    }
  },

  selectAsset: (assetId) => set({ selectedAssetId: assetId }),
  clearError: () => set({ error: null }),

  updateTask: (task: Task) => {
    const { projectFile } = get();
    if (!projectFile) return;

    const idx = projectFile.tasks.findIndex((t) => t.taskId === task.taskId);
    const newTasks = [...projectFile.tasks];
    if (idx >= 0) {
      newTasks[idx] = task;
    } else {
      newTasks.push(task);
    }

    set({
      projectFile: rebuildIndexes({ ...projectFile, tasks: newTasks }),
    });
  },
}));

// Global event subscriptions (set up once)
let _unsubscribers: Array<() => void> = [];

export async function initEventSubscriptions() {
  // Clean up previous subscriptions
  _unsubscribers.forEach((fn) => fn());
  _unsubscribers = [];

  const unsubTask = await subscribeTaskUpdates((task) => {
    useProjectStore.getState().updateTask(task);
  });
  _unsubscribers.push(unsubTask);

  const unsubProject = await subscribeProjectUpdates(() => {
    useProjectStore.getState().refreshProject();
  });
  _unsubscribers.push(unsubProject);
}
