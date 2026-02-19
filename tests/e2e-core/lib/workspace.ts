import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";

const WORKSPACE_PREFIX = "cutline-e2e-";

export class TestWorkspace {
  readonly root: string;

  constructor() {
    this.root = fs.mkdtempSync(path.join(os.tmpdir(), WORKSPACE_PREFIX));
  }

  /** Resolve `$TEMP_DIR` and other variables inside a string value. */
  resolveVars(value: string): string {
    return value.replace(/\$TEMP_DIR/g, this.root);
  }

  /** Recursively resolve variables inside an object / array / primitive. */
  resolveDeep<T>(obj: T): T {
    if (typeof obj === "string") return this.resolveVars(obj) as T;
    if (Array.isArray(obj)) return obj.map((v) => this.resolveDeep(v)) as T;
    if (obj !== null && typeof obj === "object") {
      const out: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(obj)) {
        out[k] = this.resolveDeep(v);
      }
      return out as T;
    }
    return obj;
  }

  /** Remove the temporary directory and all contents. */
  cleanup(): void {
    fs.rmSync(this.root, { recursive: true, force: true });
  }
}
