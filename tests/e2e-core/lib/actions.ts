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

  // ── Generic UI helpers ──
  waitForSelector(testId: string, timeout?: number): Promise<void>;
  getTextByTestId(testId: string): Promise<string>;
  screenshot(name: string): Promise<Buffer>;
}
