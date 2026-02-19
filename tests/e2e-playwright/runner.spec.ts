import { test } from "@playwright/test";
import { loadCases, createRunner } from "../e2e-core/lib/caseRunner";
import { PlaywrightActions } from "./actions.playwright";

const cases = loadCases();

for (const tc of cases) {
  test(tc.name, async ({ page }) => {
    const actions = new PlaywrightActions(page);
    const runner = createRunner(actions);
    await runner.execute(tc);
  });
}
