/**
 * Driver-agnostic action interface.
 * Both PlaywrightActions and WdioActions implement this contract,
 * allowing test cases to be authored once and run on either driver.
 */
export interface DriverActions {
  // ── Lifecycle ──
  launch(): Promise<void>;
  close(): Promise<void>;

  // ── Project operations ──
  clickNewProject(): Promise<void>;
  clickOpenProject(): Promise<void>;
  clickSave(): Promise<void>;
  clickImport(): Promise<void>;

  // ── Dialog handling (Tauri native dialogs) ──
  handleDirectoryPicker(dirPath: string): Promise<void>;
  handleFilePicker(filePaths: string[]): Promise<void>;
  handlePrompt(value: string): Promise<void>;

  // ── Asset operations ──
  selectAsset(index: number): Promise<void>;
  filterAssets(type: "all" | "video" | "audio" | "image"): Promise<void>;
  getAssetCount(): Promise<number>;

  // ── Task operations ──
  waitForTaskState(
    kind: string,
    state: string,
    timeout?: number
  ): Promise<void>;
  getTaskCount(state?: string): Promise<number>;
  retryTask(taskId: string): Promise<void>;
  cancelTask(taskId: string): Promise<void>;

  // ── Timeline operations (S2) ──
  clickAddToTimeline(): Promise<void>;
  clickDeleteClip(): Promise<void>;
  clickClip(clipId: string): Promise<void>;
  getClipCount(): Promise<number>;

  // ── Preview operations (S2) ──
  clickPlayPause(): Promise<void>;
  clickCaptureFrame(): Promise<void>;
  getPreviewTimeDisplay(): Promise<string>;

  // ── Marker operations (S2) ──
  switchTab(tab: string): Promise<void>;
  clickAddMarker(): Promise<void>;
  getMarkerCount(): Promise<number>;
  clickMarkerRow(index: number): Promise<void>;
  fillMarkerLabel(text: string): Promise<void>;
  fillMarkerPrompt(text: string): Promise<void>;
  clickSaveMarker(): Promise<void>;

  // ── Generic UI helpers ──
  waitMs(ms: number): Promise<void>;
  waitForSelector(testId: string, timeout?: number): Promise<void>;
  waitForSelectorHidden(testId: string, timeout?: number): Promise<void>;
  getTextByTestId(testId: string): Promise<string>;
  screenshot(name: string): Promise<Buffer>;
}
