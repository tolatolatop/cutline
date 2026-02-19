import * as fs from "node:fs";
import * as path from "node:path";

export interface ProjectFile {
  schemaVersion: string;
  project: {
    projectId: string;
    name: string;
    createdAt: string;
    updatedAt: string;
    settings: Record<string, unknown>;
    paths: Record<string, unknown>;
  };
  assets: unknown[];
  tasks: unknown[];
  timeline: Record<string, unknown>;
  exports: unknown[];
  indexes: Record<string, unknown>;
}

/** Read and parse a project.json from the given project directory. */
export function readProjectJson(projectDir: string): ProjectFile {
  const filePath = path.join(projectDir, "project.json");
  if (!fs.existsSync(filePath)) {
    throw new Error(`project.json not found at ${filePath}`);
  }
  return JSON.parse(fs.readFileSync(filePath, "utf-8"));
}

/** Assert the project.json has a valid top-level structure. */
export function assertProjectStructure(project: ProjectFile): void {
  const required: (keyof ProjectFile)[] = [
    "schemaVersion",
    "project",
    "assets",
    "tasks",
    "timeline",
    "exports",
    "indexes",
  ];
  for (const key of required) {
    if (!(key in project)) {
      throw new Error(`project.json missing required key: "${key}"`);
    }
  }

  if (!project.project.projectId) {
    throw new Error("project.json project.projectId is empty");
  }
  if (!project.project.name) {
    throw new Error("project.json project.name is empty");
  }
}

/** Assert that the project has the expected number of assets. */
export function assertAssetCount(
  project: ProjectFile,
  expected: number
): void {
  if (project.assets.length !== expected) {
    throw new Error(
      `Expected ${expected} assets, got ${project.assets.length}`
    );
  }
}

/** Assert that the project has the expected number of tasks. */
export function assertTaskCount(
  project: ProjectFile,
  expected: number
): void {
  if (project.tasks.length !== expected) {
    throw new Error(
      `Expected ${expected} tasks, got ${project.tasks.length}`
    );
  }
}
