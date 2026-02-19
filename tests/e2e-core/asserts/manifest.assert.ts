import * as fs from "node:fs";
import * as path from "node:path";

/** Expected workspace subdirectories created by Cutline on project init. */
const EXPECTED_WORKSPACE_DIRS = [
  "workspace",
  "workspace/assets",
  "workspace/assets/video",
  "workspace/assets/audio",
  "workspace/assets/images",
  "workspace/cache",
  "workspace/cache/thumbs",
  "workspace/cache/proxy",
  "workspace/exports",
];

/**
 * Verify that the workspace directory structure is correctly created
 * under the given project root.
 */
export function assertWorkspaceStructure(projectDir: string): void {
  for (const rel of EXPECTED_WORKSPACE_DIRS) {
    const full = path.join(projectDir, rel);
    if (!fs.existsSync(full)) {
      throw new Error(`Missing workspace directory: ${rel} (expected at ${full})`);
    }
    const stat = fs.statSync(full);
    if (!stat.isDirectory()) {
      throw new Error(`${rel} exists but is not a directory`);
    }
  }
}

/** Assert that a project.json file exists in the project root. */
export function assertProjectJsonExists(projectDir: string): void {
  const p = path.join(projectDir, "project.json");
  if (!fs.existsSync(p)) {
    throw new Error(`project.json not found at ${p}`);
  }
}

/**
 * Assert that imported asset files exist under the workspace/assets subtree.
 * @param projectDir - Root project directory
 * @param relativePaths - Array of paths relative to project root
 *   (e.g. "workspace/assets/video/sample.mp4")
 */
export function assertAssetFilesExist(
  projectDir: string,
  relativePaths: string[]
): void {
  for (const rel of relativePaths) {
    const full = path.join(projectDir, rel);
    if (!fs.existsSync(full)) {
      throw new Error(`Asset file missing: ${rel}`);
    }
  }
}
