import { loadCases, createRunner } from "../e2e-core/lib/caseRunner";
import { WdioActions } from "./actions.wdio";

const cases = loadCases();

describe("Cutline E2E (WDIO + tauri-driver)", () => {
  for (const tc of cases) {
    it(tc.name, async () => {
      const actions = new WdioActions();
      const runner = createRunner(actions);
      await runner.execute(tc);
    });
  }
});
