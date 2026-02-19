import type { Page } from "@playwright/test";
import * as path from "node:path";
import type { DriverActions } from "../e2e-core/lib/actions";
import { buildMockScript } from "./tauri-mock";

const FIXTURES_DIR = path.resolve(__dirname, "../e2e-core/fixtures");

function testIdSelector(testId: string): string {
  return `[data-testid="${testId}"]`;
}

export class PlaywrightActions implements DriverActions {
  constructor(private readonly page: Page) {}

  async launch(): Promise<void> {
    await this.page.addInitScript({ content: buildMockScript() });
    await this.page.goto("/");
    await this.page.waitForLoadState("networkidle");
  }

  async close(): Promise<void> {
    // Playwright manages page lifecycle — nothing to do
  }

  // ── Project operations ──

  async clickNewProject(): Promise<void> {
    await this.page.click(testIdSelector("btn-new-project"));
  }

  async clickOpenProject(): Promise<void> {
    await this.page.click(testIdSelector("btn-open-project"));
  }

  async clickSave(): Promise<void> {
    await this.page.click(testIdSelector("btn-save"));
  }

  async clickImport(): Promise<void> {
    await this.page.click(testIdSelector("btn-import"));
  }

  // ── Dialog handling ──
  // In mock mode, dialogs are intercepted by the Tauri mock layer.
  // We simulate the dialog result by calling the mock handler directly.

  async handleDirectoryPicker(dirPath: string): Promise<void> {
    await this.page.evaluate((dir) => {
      (window as any).__TAURI_MOCK_DIALOG_DIR__ = dir;
    }, dirPath);
  }

  async handleFilePicker(fixturePaths: string[]): Promise<void> {
    const absolutePaths = fixturePaths.map((f) =>
      path.resolve(FIXTURES_DIR, f)
    );
    await this.page.evaluate((paths) => {
      (window as any).__TAURI_MOCK_DIALOG_FILES__ = paths;
    }, absolutePaths);
  }

  async handlePrompt(value: string): Promise<void> {
    this.page.once("dialog", async (dialog) => {
      await dialog.accept(value);
    });
  }

  // ── Asset operations ──

  async selectAsset(index: number): Promise<void> {
    const card = this.page.locator(testIdSelector(`asset-card-${index}`));
    await card.click();
  }

  async filterAssets(
    type: "all" | "video" | "audio" | "image"
  ): Promise<void> {
    await this.page.click(testIdSelector(`asset-filter-${type}`));
  }

  async getAssetCount(): Promise<number> {
    const cards = this.page.locator('[data-testid^="asset-card-"]');
    return cards.count();
  }

  // ── Task operations ──

  async waitForTaskState(
    kind: string,
    state: string,
    timeout = 30000
  ): Promise<void> {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      const matched = await this.page.evaluate(
        ({ kind: k, state: s }) => {
          const panel = document.querySelector('[data-testid="task-panel"]');
          if (!panel) return false;
          const rows = panel.querySelectorAll("[class*='border-b']");
          for (const row of rows) {
            const text = row.textContent || "";
            const kindMap: Record<string, string> = {
              probe: "媒体探测",
              thumb: "缩略图",
              proxy: "代理视频",
              generate: "AI 生成",
              export: "导出",
            };
            const stateMap: Record<string, string> = {
              queued: "排队中",
              running: "执行中",
              succeeded: "已完成",
              failed: "失败",
              canceled: "已取消",
            };
            if (
              text.includes(kindMap[k] || k) &&
              text.includes(stateMap[s] || s)
            ) {
              return true;
            }
          }
          return false;
        },
        { kind, state }
      );
      if (matched) return;
      await this.page.waitForTimeout(500);
    }
    throw new Error(
      `Timed out waiting for task "${kind}" to reach state "${state}"`
    );
  }

  async getTaskCount(state?: string): Promise<number> {
    if (!state || state === "all") {
      const rows = this.page.locator(
        '[data-testid="task-panel"] [class*="border-b border-zinc-800"]'
      );
      return rows.count();
    }

    return this.page.evaluate((s) => {
      const stateMap: Record<string, string> = {
        queued: "排队中",
        running: "执行中",
        succeeded: "已完成",
        failed: "失败",
        canceled: "已取消",
      };
      const label = stateMap[s] || s;
      const panel = document.querySelector('[data-testid="task-panel"]');
      if (!panel) return 0;
      const rows = panel.querySelectorAll("[class*='border-b']");
      let count = 0;
      for (const row of rows) {
        if ((row.textContent || "").includes(label)) count++;
      }
      return count;
    }, state);
  }

  async retryTask(taskId: string): Promise<void> {
    await this.page.click(testIdSelector(`task-retry-${taskId}`));
  }

  async cancelTask(taskId: string): Promise<void> {
    await this.page.click(testIdSelector(`task-cancel-${taskId}`));
  }

  // ── Generic UI helpers ──

  async waitForSelector(testId: string, timeout = 5000): Promise<void> {
    await this.page.waitForSelector(testIdSelector(testId), { timeout });
  }

  async getTextByTestId(testId: string): Promise<string> {
    const el = this.page.locator(testIdSelector(testId));
    return (await el.textContent()) ?? "";
  }

  async screenshot(name: string): Promise<Buffer> {
    return (await this.page.screenshot({
      fullPage: true,
    })) as Buffer;
  }
}
