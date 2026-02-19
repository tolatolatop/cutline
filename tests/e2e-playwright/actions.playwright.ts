import type { Page } from "@playwright/test";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import type { DriverActions } from "../e2e-core/lib/actions";
import { buildMockScript } from "./tauri-mock";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
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
    return this.page.evaluate((s) => {
      const panel = document.querySelector('[data-testid="task-panel"]');
      if (!panel) return 0;
      // Task rows live inside the scrollable container (last child with overflow-y-auto)
      const scrollArea = panel.querySelector(".overflow-y-auto");
      if (!scrollArea) return 0;
      const rows = scrollArea.querySelectorAll(":scope > [class*='border-b']");
      if (!s || s === "all") return rows.length;

      const stateMap: Record<string, string> = {
        queued: "排队中",
        running: "执行中",
        succeeded: "已完成",
        failed: "失败",
        canceled: "已取消",
      };
      const label = stateMap[s] || s;
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

  // ── Timeline operations (S2) ──

  async clickAddToTimeline(): Promise<void> {
    await this.page.click(testIdSelector("btn-add-to-timeline"));
  }

  async clickDeleteClip(): Promise<void> {
    await this.page.click(testIdSelector("btn-delete-clip"));
  }

  async clickClip(clipId: string): Promise<void> {
    await this.page.click(testIdSelector(`clip-block-${clipId}`));
  }

  async clickClipByIndex(index: number): Promise<void> {
    const clips = this.page.locator('[data-testid^="clip-block-"]');
    await clips.nth(index).click();
  }

  async dragClipByIndex(index: number, deltaXPx: number): Promise<void> {
    const clip = this.page.locator('[data-testid^="clip-block-"]').nth(index);
    const box = await clip.boundingBox();
    if (!box) throw new Error(`Clip at index ${index} not found or not visible`);
    const startX = box.x + box.width / 2;
    const startY = box.y + box.height / 2;
    await this.page.mouse.move(startX, startY);
    await this.page.mouse.down();
    await this.page.mouse.move(startX + deltaXPx, startY, { steps: 5 });
    await this.page.mouse.up();
    await this.page.waitForTimeout(100);
  }

  async trimClipByIndex(
    index: number,
    side: "left" | "right",
    deltaXPx: number
  ): Promise<void> {
    const clip = this.page.locator('[data-testid^="clip-block-"]').nth(index);
    const box = await clip.boundingBox();
    if (!box) throw new Error(`Clip at index ${index} not found or not visible`);
    const handleX = side === "left" ? box.x + 3 : box.x + box.width - 3;
    const handleY = box.y + box.height / 2;
    await this.page.mouse.move(handleX, handleY);
    await this.page.mouse.down();
    await this.page.mouse.move(handleX + deltaXPx, handleY, { steps: 5 });
    await this.page.mouse.up();
    await this.page.waitForTimeout(100);
  }

  async getClipCount(): Promise<number> {
    const clips = this.page.locator('[data-testid^="clip-block-"]');
    return clips.count();
  }

  async getClipLeftPx(index: number): Promise<number> {
    const clip = this.page.locator('[data-testid^="clip-block-"]').nth(index);
    const style = await clip.getAttribute("style");
    const match = style?.match(/left:\s*([\d.]+)px/);
    return match ? parseFloat(match[1]) : 0;
  }

  async getClipWidthPx(index: number): Promise<number> {
    const clip = this.page.locator('[data-testid^="clip-block-"]').nth(index);
    const box = await clip.boundingBox();
    return box?.width ?? 0;
  }

  async clickZoom(level: number): Promise<void> {
    await this.page.click(testIdSelector(`btn-zoom-${level}`));
  }

  // ── Preview operations (S2) ──

  async clickPlayPause(): Promise<void> {
    await this.page.click(testIdSelector("btn-play-pause"));
  }

  async clickCaptureFrame(): Promise<void> {
    await this.page.click(testIdSelector("btn-capture-frame"));
  }

  async getPreviewTimeDisplay(): Promise<string> {
    const el = this.page.locator(testIdSelector("preview-time-display"));
    return (await el.textContent()) ?? "";
  }

  // ── Marker operations (S2) ──

  async switchTab(tab: string): Promise<void> {
    await this.page.click(testIdSelector(`tab-${tab}`));
  }

  async clickAddMarker(): Promise<void> {
    await this.page.click(testIdSelector("btn-add-marker"));
  }

  async getMarkerCount(): Promise<number> {
    const panel = this.page.locator(testIdSelector("marker-list"));
    const rows = panel.locator('[data-testid^="marker-row-"]');
    return rows.count();
  }

  async clickMarkerRow(index: number): Promise<void> {
    const panel = this.page.locator(testIdSelector("marker-list"));
    const rows = panel.locator('[data-testid^="marker-row-"]');
    await rows.nth(index).click();
  }

  async doubleClickMarkerRow(index: number): Promise<void> {
    const panel = this.page.locator(testIdSelector("marker-list"));
    const rows = panel.locator('[data-testid^="marker-row-"]');
    await rows.nth(index).dblclick();
  }

  async deleteMarkerByIndex(index: number): Promise<void> {
    const panel = this.page.locator(testIdSelector("marker-list"));
    const rows = panel.locator('[data-testid^="marker-row-"]');
    const deleteBtn = rows.nth(index).locator('[data-testid^="btn-delete-marker-"]');
    await deleteBtn.click();
  }

  async fillMarkerLabel(text: string): Promise<void> {
    const input = this.page.locator(testIdSelector("marker-edit-label"));
    await input.fill(text);
  }

  async fillMarkerPrompt(text: string): Promise<void> {
    const textarea = this.page.locator(testIdSelector("marker-edit-prompt"));
    await textarea.fill(text);
  }

  async clickSaveMarker(): Promise<void> {
    await this.page.click(testIdSelector("btn-save-marker"));
  }

  // ── Generic UI helpers ──

  async waitMs(ms: number): Promise<void> {
    await this.page.waitForTimeout(ms);
  }

  async waitForSelector(testId: string, timeout = 5000): Promise<void> {
    await this.page.waitForSelector(testIdSelector(testId), { timeout });
  }

  async waitForSelectorHidden(testId: string, timeout = 5000): Promise<void> {
    await this.page.waitForSelector(testIdSelector(testId), {
      state: "hidden",
      timeout,
    });
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
