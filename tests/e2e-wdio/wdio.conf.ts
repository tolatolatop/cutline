import type { Options } from "@wdio/types";
import * as path from "node:path";
import { spawn, type ChildProcess } from "node:child_process";

let tauriDriver: ChildProcess | undefined;

export const config: Options.Testrunner = {
  runner: "local",
  autoCompileOpts: {
    tsNodeOpts: { project: path.resolve(__dirname, "../tsconfig.json") },
  },
  specs: [path.resolve(__dirname, "./runner.spec.ts")],
  maxInstances: 1,
  capabilities: [
    {
      // @ts-expect-error tauri capabilities are not in the official types
      "tauri:options": {
        application: path.resolve(
          __dirname,
          "../../src-tauri/target/release/cutline.exe"
        ),
      },
    },
  ],
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 120_000,
  },

  onPrepare() {
    tauriDriver = spawn("tauri-driver", [], {
      stdio: ["ignore", "pipe", "pipe"],
    });
    tauriDriver.stdout?.on("data", (data: Buffer) => {
      console.log(`[tauri-driver] ${data.toString().trim()}`);
    });
    tauriDriver.stderr?.on("data", (data: Buffer) => {
      console.error(`[tauri-driver] ${data.toString().trim()}`);
    });

    return new Promise<void>((resolve) => {
      // Give tauri-driver time to start
      setTimeout(resolve, 2000);
    });
  },

  onComplete() {
    if (tauriDriver) {
      tauriDriver.kill();
      tauriDriver = undefined;
    }
  },
};
