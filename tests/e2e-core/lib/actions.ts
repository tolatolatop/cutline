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
  clickClipByIndex(index: number): Promise<void>;
  dragClipByIndex(index: number, deltaXPx: number): Promise<void>;
  trimClipByIndex(index: number, side: "left" | "right", deltaXPx: number): Promise<void>;
  getClipCount(): Promise<number>;
  getClipLeftPx(index: number): Promise<number>;
  getClipWidthPx(index: number): Promise<number>;
  clickZoom(level: number): Promise<void>;

  // ── Preview operations (S2) ──
  clickPlayPause(): Promise<void>;
  clickCaptureFrame(): Promise<void>;
  getPreviewTimeDisplay(): Promise<string>;

  // ── Marker operations (S2) ──
  switchTab(tab: string): Promise<void>;
  clickAddMarker(): Promise<void>;
  getMarkerCount(): Promise<number>;
  clickMarkerRow(index: number): Promise<void>;
  doubleClickMarkerRow(index: number): Promise<void>;
  deleteMarkerByIndex(index: number): Promise<void>;
  fillMarkerLabel(text: string): Promise<void>;
  fillMarkerPrompt(text: string): Promise<void>;
  clickSaveMarker(): Promise<void>;

  // ── Settings / Provider operations (S3) ──
  clickSettings(): Promise<void>;
  closeSettings(): Promise<void>;
  clickAddProvider(): Promise<void>;
  clickProviderItem(name: string): Promise<void>;
  fillProviderName(name: string): Promise<void>;
  fillDisplayName(name: string): Promise<void>;
  fillBaseUrl(url: string): Promise<void>;
  selectAuthKind(kind: "api_key" | "session_cookie"): Promise<void>;
  fillCredentialRef(profileName: string, value: string): Promise<void>;
  clickSaveProvider(): Promise<void>;
  clickDeleteProvider(): Promise<void>;
  clickConfirmDeleteProvider(): Promise<void>;
  fillSecret(credentialRef: string, value: string): Promise<void>;
  clickConnect(credentialRef: string): Promise<void>;
  clickDisconnect(credentialRef: string): Promise<void>;
  clickTestProfile(profileName: string): Promise<void>;
  getProviderCount(): Promise<number>;
  getConnectionStatus(credentialRef: string): Promise<string>;
  getTestResultText(): Promise<string>;

  // ── Generic UI helpers ──
  waitMs(ms: number): Promise<void>;
  waitForSelector(testId: string, timeout?: number): Promise<void>;
  waitForSelectorHidden(testId: string, timeout?: number): Promise<void>;
  getTextByTestId(testId: string): Promise<string>;
  screenshot(name: string): Promise<Buffer>;
}
