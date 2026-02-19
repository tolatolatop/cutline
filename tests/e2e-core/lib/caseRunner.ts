import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import type { DriverActions } from "./actions";
import { TestWorkspace } from "./workspace";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// ── Case JSON schema ──

export interface CaseStep {
  action: string;
  params?: Record<string, unknown>;
}

export interface TestCase {
  id: string;
  name: string;
  description?: string;
  steps: CaseStep[];
}

// ── Built-in assertion handlers ──

type AssertHandler = (
  actions: DriverActions,
  params: Record<string, unknown>
) => Promise<void>;

const builtinAsserts: Record<string, AssertHandler> = {
  async assertAssetCount(actions, params) {
    const expected = params.expected as number;
    const actual = await actions.getAssetCount();
    if (actual !== expected) {
      throw new Error(
        `assertAssetCount: expected ${expected}, got ${actual}`
      );
    }
  },

  async assertTaskCount(actions, params) {
    const expected = params.expected as number;
    const state = params.state as string | undefined;
    const actual = await actions.getTaskCount(state);
    if (actual !== expected) {
      throw new Error(
        `assertTaskCount(${state ?? "all"}): expected ${expected}, got ${actual}`
      );
    }
  },

  async assertClipCount(actions, params) {
    const expected = params.expected as number;
    const actual = await actions.getClipCount();
    if (actual !== expected) {
      throw new Error(
        `assertClipCount: expected ${expected}, got ${actual}`
      );
    }
  },

  async assertMarkerCount(actions, params) {
    const expected = params.expected as number;
    const actual = await actions.getMarkerCount();
    if (actual !== expected) {
      throw new Error(
        `assertMarkerCount: expected ${expected}, got ${actual}`
      );
    }
  },

  async assertClipLeftGreaterThan(actions, params) {
    const index = (params.index as number) ?? 0;
    const minPx = params.minPx as number;
    const actual = await actions.getClipLeftPx(index);
    if (actual <= minPx) {
      throw new Error(
        `assertClipLeftGreaterThan: expected clip ${index} left > ${minPx}, got ${actual}`
      );
    }
  },

  async assertClipWidthChanged(actions, params) {
    const index = (params.index as number) ?? 0;
    const originalPx = params.originalPx as number;
    const actual = await actions.getClipWidthPx(index);
    if (Math.abs(actual - originalPx) < 2) {
      throw new Error(
        `assertClipWidthChanged: expected clip ${index} width to differ from ${originalPx}, got ${actual}`
      );
    }
  },

  async assertSelectorVisible(actions, params) {
    const testId = params.testId as string;
    await actions.waitForSelector(testId, (params.timeout as number) ?? 5000);
  },

  async assertSelectorHidden(actions, params) {
    const testId = params.testId as string;
    await actions.waitForSelectorHidden(
      testId,
      (params.timeout as number) ?? 5000
    );
  },

  async assertTextContains(actions, params) {
    const testId = params.testId as string;
    const substring = params.contains as string;
    const text = await actions.getTextByTestId(testId);
    if (!text.includes(substring)) {
      throw new Error(
        `assertTextContains: "${testId}" text does not contain "${substring}". Actual: "${text}"`
      );
    }
  },

  async assertPreviewTimeContains(actions, params) {
    const substring = params.contains as string;
    const text = await actions.getPreviewTimeDisplay();
    if (!text.includes(substring)) {
      throw new Error(
        `assertPreviewTimeContains: preview time display does not contain "${substring}". Actual: "${text}"`
      );
    }
  },

  async assertProviderCount(actions, params) {
    const expected = params.expected as number;
    const actual = await actions.getProviderCount();
    if (actual !== expected) {
      throw new Error(
        `assertProviderCount: expected ${expected}, got ${actual}`
      );
    }
  },

  async assertConnectionStatus(actions, params) {
    const credentialRef = params.credentialRef as string;
    const expected = params.expected as string;
    const actual = await actions.getConnectionStatus(credentialRef);
    if (actual !== expected) {
      throw new Error(
        `assertConnectionStatus(${credentialRef}): expected "${expected}", got "${actual}"`
      );
    }
  },

  async assertTestResultContains(actions, params) {
    const substring = params.contains as string;
    const text = await actions.getTestResultText();
    if (!text.includes(substring)) {
      throw new Error(
        `assertTestResultContains: test result does not contain "${substring}". Actual: "${text}"`
      );
    }
  },
};

// ── Case loader ──

const CASES_DIR = path.resolve(__dirname, "../cases");

export function loadCases(dir?: string): TestCase[] {
  const casesDir = dir ?? CASES_DIR;
  const files = fs.readdirSync(casesDir).filter((f) => f.endsWith(".case.json"));
  return files.map((f) => {
    const raw = fs.readFileSync(path.join(casesDir, f), "utf-8");
    return JSON.parse(raw) as TestCase;
  });
}

export function loadCase(id: string, dir?: string): TestCase {
  const casesDir = dir ?? CASES_DIR;
  const file = `${id}.case.json`;
  const raw = fs.readFileSync(path.join(casesDir, file), "utf-8");
  return JSON.parse(raw) as TestCase;
}

// ── Runner ──

export interface CaseRunner {
  execute(tc: TestCase): Promise<void>;
}

export function createRunner(actions: DriverActions): CaseRunner {
  const workspace = new TestWorkspace();

  return {
    async execute(tc: TestCase) {
      try {
        for (const step of tc.steps) {
          const resolvedParams = step.params
            ? workspace.resolveDeep(step.params)
            : {};

          if (step.action in builtinAsserts) {
            await builtinAsserts[step.action](actions, resolvedParams);
            continue;
          }

          const method = (actions as unknown as Record<string, Function>)[
            step.action
          ];
          if (typeof method !== "function") {
            throw new Error(
              `Unknown action "${step.action}" in case "${tc.id}"`
            );
          }

          const args = paramsToArgs(step.action, resolvedParams);
          await method.apply(actions, args);
        }
      } finally {
        workspace.cleanup();
      }
    },
  };
}

/**
 * Convert a flat params object to a positional argument list
 * based on the known signatures of DriverActions methods.
 */
function paramsToArgs(
  action: string,
  params: Record<string, unknown>
): unknown[] {
  const signatures: Record<string, string[]> = {
    handleDirectoryPicker: ["dir"],
    handleFilePicker: ["fixtures"],
    handlePrompt: ["value"],
    selectAsset: ["index"],
    filterAssets: ["type"],
    waitForTaskState: ["kind", "state", "timeout"],
    retryTask: ["taskId"],
    cancelTask: ["taskId"],
    clickClip: ["clipId"],
    clickClipByIndex: ["index"],
    dragClipByIndex: ["index", "deltaXPx"],
    trimClipByIndex: ["index", "side", "deltaXPx"],
    clickZoom: ["level"],
    switchTab: ["tab"],
    clickMarkerRow: ["index"],
    doubleClickMarkerRow: ["index"],
    deleteMarkerByIndex: ["index"],
    fillMarkerLabel: ["text"],
    fillMarkerPrompt: ["text"],
    clickProviderItem: ["name"],
    fillProviderName: ["name"],
    fillDisplayName: ["name"],
    fillBaseUrl: ["url"],
    selectAuthKind: ["kind"],
    fillCredentialRef: ["profileName", "value"],
    fillSecret: ["credentialRef", "value"],
    clickConnect: ["credentialRef"],
    clickDisconnect: ["credentialRef"],
    clickTestProfile: ["profileName"],
    waitMs: ["ms"],
    waitForSelector: ["testId", "timeout"],
    waitForSelectorHidden: ["testId", "timeout"],
    getTextByTestId: ["testId"],
    screenshot: ["name"],
  };

  const sig = signatures[action];
  if (!sig) return [];
  return sig.map((key) => params[key]).filter((v) => v !== undefined);
}
