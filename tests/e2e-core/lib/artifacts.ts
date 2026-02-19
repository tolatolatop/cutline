import * as fs from "node:fs";
import * as path from "node:path";

const DEFAULT_DIR = path.resolve(__dirname, "../../test-artifacts");

export class ArtifactManager {
  private readonly dir: string;

  constructor(baseDir?: string) {
    this.dir = baseDir ?? DEFAULT_DIR;
    fs.mkdirSync(this.dir, { recursive: true });
  }

  /** Persist a screenshot buffer with a descriptive name. */
  saveScreenshot(name: string, data: Buffer): string {
    const sanitized = name.replace(/[^a-zA-Z0-9_-]/g, "_");
    const filePath = path.join(
      this.dir,
      `${sanitized}-${Date.now()}.png`
    );
    fs.writeFileSync(filePath, data);
    return filePath;
  }

  /** Write arbitrary text (logs, JSON dumps, etc.) to the artifacts dir. */
  saveLog(name: string, content: string): string {
    const sanitized = name.replace(/[^a-zA-Z0-9_-]/g, "_");
    const filePath = path.join(this.dir, `${sanitized}-${Date.now()}.log`);
    fs.writeFileSync(filePath, content, "utf-8");
    return filePath;
  }

  get directory(): string {
    return this.dir;
  }
}
