import * as path from "node:path";
import type { DriverActions } from "../e2e-core/lib/actions";

const FIXTURES_DIR = path.resolve(__dirname, "../e2e-core/fixtures");

function testIdSelector(testId: string): string {
  return `[data-testid="${testId}"]`;
}

export class WdioActions implements DriverActions {
  async launch(): Promise<void> {
    // tauri-driver launches the app automatically via capabilities.
    // Wait for the window to be ready.
    await browser.waitUntil(
      async () => (await browser.getTitle()) !== "",
      { timeout: 15_000, timeoutMsg: "App window did not appear" }
    );
  }

  async close(): Promise<void> {
    await browser.deleteSession();
  }

  // ── Project operations ──

  async clickNewProject(): Promise<void> {
    const btn = await browser.$(testIdSelector("btn-new-project"));
    await btn.waitForClickable({ timeout: 5000 });
    await btn.click();
  }

  async clickOpenProject(): Promise<void> {
    const btn = await browser.$(testIdSelector("btn-open-project"));
    await btn.waitForClickable({ timeout: 5000 });
    await btn.click();
  }

  async clickSave(): Promise<void> {
    const btn = await browser.$(testIdSelector("btn-save"));
    await btn.waitForClickable({ timeout: 5000 });
    await btn.click();
  }

  async clickImport(): Promise<void> {
    const btn = await browser.$(testIdSelector("btn-import"));
    await btn.waitForClickable({ timeout: 5000 });
    await btn.click();
  }

  // ── Dialog handling ──
  // With tauri-driver, native dialogs are NOT directly automatable.
  // These methods set up the test by injecting values via executeScript
  // so the app can read them in place of real dialog results.

  async handleDirectoryPicker(dirPath: string): Promise<void> {
    await browser.execute((dir: string) => {
      (window as any).__TAURI_TEST_DIR_PICK__ = dir;
    }, dirPath);
  }

  async handleFilePicker(fixturePaths: string[]): Promise<void> {
    const absolutePaths = fixturePaths.map((f) =>
      path.resolve(FIXTURES_DIR, f)
    );
    await browser.execute((paths: string[]) => {
      (window as any).__TAURI_TEST_FILE_PICK__ = paths;
    }, absolutePaths);
  }

  async handlePrompt(value: string): Promise<void> {
    await browser.execute((val: string) => {
      (window as any).__TAURI_TEST_PROMPT__ = val;
    }, value);
  }

  // ── Asset operations ──

  async selectAsset(index: number): Promise<void> {
    const card = await browser.$(testIdSelector(`asset-card-${index}`));
    await card.waitForDisplayed({ timeout: 5000 });
    await card.click();
  }

  async filterAssets(
    type: "all" | "video" | "audio" | "image"
  ): Promise<void> {
    const btn = await browser.$(testIdSelector(`asset-filter-${type}`));
    await btn.waitForClickable({ timeout: 5000 });
    await btn.click();
  }

  async getAssetCount(): Promise<number> {
    const cards = await browser.$$('[data-testid^="asset-card-"]');
    return cards.length;
  }

  // ── Task operations ──

  async waitForTaskState(
    kind: string,
    state: string,
    timeout = 30000
  ): Promise<void> {
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

    const kindLabel = kindMap[kind] || kind;
    const stateLabel = stateMap[state] || state;

    await browser.waitUntil(
      async () => {
        const panel = await browser.$(testIdSelector("task-panel"));
        if (!(await panel.isExisting())) return false;
        const text = await panel.getText();
        return text.includes(kindLabel) && text.includes(stateLabel);
      },
      { timeout, timeoutMsg: `Task "${kind}" did not reach state "${state}"` }
    );
  }

  async getTaskCount(state?: string): Promise<number> {
    const panel = await browser.$(testIdSelector("task-panel"));
    if (!(await panel.isExisting())) return 0;

    if (!state || state === "all") {
      const rows = await panel.$$("[class*='border-b border-zinc-800']");
      return rows.length;
    }

    const stateMap: Record<string, string> = {
      queued: "排队中",
      running: "执行中",
      succeeded: "已完成",
      failed: "失败",
      canceled: "已取消",
    };
    const label = stateMap[state] || state;

    return browser.execute(
      (selector: string, lbl: string) => {
        const el = document.querySelector(selector);
        if (!el) return 0;
        const rows = el.querySelectorAll("[class*='border-b']");
        let count = 0;
        for (const row of rows) {
          if ((row.textContent || "").includes(lbl)) count++;
        }
        return count;
      },
      testIdSelector("task-panel"),
      label
    );
  }

  async retryTask(taskId: string): Promise<void> {
    const btn = await browser.$(testIdSelector(`task-retry-${taskId}`));
    await btn.waitForClickable({ timeout: 5000 });
    await btn.click();
  }

  async cancelTask(taskId: string): Promise<void> {
    const btn = await browser.$(testIdSelector(`task-cancel-${taskId}`));
    await btn.waitForClickable({ timeout: 5000 });
    await btn.click();
  }

  // ── Generic UI helpers ──

  async waitForSelector(testId: string, timeout = 5000): Promise<void> {
    const el = await browser.$(testIdSelector(testId));
    await el.waitForExist({ timeout });
  }

  async getTextByTestId(testId: string): Promise<string> {
    const el = await browser.$(testIdSelector(testId));
    return el.getText();
  }

  async screenshot(name: string): Promise<Buffer> {
    const data = await browser.takeScreenshot();
    return Buffer.from(data, "base64");
  }
}
