/**
 * Tauri API mock layer for Playwright tests.
 *
 * When testing against the Vite dev server (not the Tauri binary),
 * `window.__TAURI_INTERNALS__` is undefined. We inject this mock
 * via `page.addInitScript()` so that the frontend's `invoke()` calls
 * are routed to deterministic mock data.
 */

import { v4 as uuid } from "uuid";

// Re-export a serialisable mock factory (no closures over Node APIs).
// This function body will be stringified and injected into the browser.

export function buildMockScript(): string {
  // The entire function below runs in the BROWSER context.
  return `
(function () {
  const assets = [];
  const tasks = [];
  let project = null;

  function makeId() {
    return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, function (c) {
      var r = (Math.random() * 16) | 0;
      return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
    });
  }

  function now() {
    return new Date().toISOString();
  }

  function makeProject(name) {
    const pid = makeId();
    return {
      schemaVersion: "0.1.0",
      project: {
        projectId: pid,
        name: name,
        createdAt: now(),
        updatedAt: now(),
        settings: {
          fps: 24,
          resolution: { width: 1920, height: 1080 },
          aspectRatio: "16:9",
          sampleRate: 48000,
        },
        paths: {
          workspaceRoot: "workspace",
          assetsDir: "workspace/assets",
          cacheDir: "workspace/cache",
          exportsDir: "workspace/exports",
        },
        timelineId: makeId(),
        defaultDraftTrackIds: { video: makeId(), audio: makeId(), text: makeId() },
      },
      assets: assets,
      tasks: tasks,
      timeline: {
        timelineId: makeId(),
        timebase: { fps: 24, unit: "frames" },
        tracks: [],
      },
      exports: [],
      indexes: { assetById: {}, taskById: {} },
    };
  }

  function guessType(path) {
    var ext = path.split(".").pop().toLowerCase();
    if (["mp4","mov","avi","mkv","webm","flv","wmv"].includes(ext)) return "video";
    if (["mp3","wav","aac","flac","ogg","wma"].includes(ext)) return "audio";
    return "image";
  }

  function makeMeta(type) {
    if (type === "video") {
      return { kind: "video", container: "mp4", codec: "h264", durationSec: 1.0, width: 320, height: 240, fps: 24, audio: { present: false, sampleRate: 0, channels: 0 } };
    }
    if (type === "audio") {
      return { kind: "audio", codec: "mp3", durationSec: 1.0, sampleRate: 44100, channels: 1 };
    }
    return { kind: "image", format: "png", width: 4, height: 4 };
  }

  function importAsset(filePath) {
    var type = guessType(filePath);
    var asset = {
      assetId: makeId(),
      type: type,
      source: "uploaded",
      fingerprint: { algo: "sha256", value: makeId() + makeId(), basis: "file_bytes" },
      path: "workspace/assets/" + type + "/" + filePath.split(/[\\\\/]/).pop(),
      meta: makeMeta(type),
      tags: [],
      createdAt: now(),
    };
    assets.push(asset);
    project.indexes.assetById[asset.assetId] = assets.length - 1;

    // Create thumb task
    var thumbTask = {
      taskId: makeId(),
      kind: "thumb",
      state: "succeeded",
      createdAt: now(),
      updatedAt: now(),
      input: { assetId: asset.assetId },
      output: {},
      progress: { phase: "done", percent: 100 },
      retries: { count: 0, max: 3 },
      deps: [],
      events: [{ t: now(), level: "info", msg: "Thumbnail generated" }],
    };
    tasks.push(thumbTask);

    if (type === "video") {
      var proxyTask = {
        taskId: makeId(),
        kind: "proxy",
        state: "succeeded",
        createdAt: now(),
        updatedAt: now(),
        input: { assetId: asset.assetId },
        output: {},
        progress: { phase: "done", percent: 100 },
        retries: { count: 0, max: 3 },
        deps: [thumbTask.taskId],
        events: [{ t: now(), level: "info", msg: "Proxy generated" }],
      };
      tasks.push(proxyTask);
    }

    return asset;
  }

  const handlers = {
    create_project: function (args) {
      project = makeProject(args.name || "Untitled");
      return project;
    },
    open_project: function () {
      if (!project) project = makeProject("Opened Project");
      return project;
    },
    save_project: function () {
      return null;
    },
    get_project: function () {
      return project;
    },
    import_assets: function (args) {
      var paths = args.filePaths || [];
      return paths.map(function (p) { return importAsset(p); });
    },
    probe_media: function () {
      return {};
    },
    read_file_base64: function () {
      return "";
    },
    task_enqueue: function () {
      return makeId();
    },
    task_retry: function () {
      return null;
    },
    task_cancel: function () {
      return null;
    },
    task_list: function () {
      return tasks.map(function (t) {
        return {
          taskId: t.taskId,
          kind: t.kind,
          state: t.state,
          createdAt: t.createdAt,
          updatedAt: t.updatedAt,
          progress: t.progress,
          error: t.error,
          retries: t.retries,
        };
      });
    },
  };

  if (!window.__TAURI_INTERNALS__) {
    window.__TAURI_INTERNALS__ = {
      invoke: function (cmd, args) {
        var handler = handlers[cmd];
        if (handler) {
          return Promise.resolve(handler(args || {}));
        }
        console.warn("[tauri-mock] unhandled command:", cmd, args);
        return Promise.resolve(null);
      },
      transformCallback: function (callback) {
        var id = Math.random();
        window["_" + id] = callback;
        return id;
      },
      metadata: { currentWebview: { label: "main" }, currentWindow: { label: "main" } },
    };
  }

  // Also mock the event listener to prevent errors
  if (!window.__TAURI_INTERNALS__.invoke.__patched) {
    var origInvoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = function (cmd, args) {
      if (cmd === "plugin:event|listen" || cmd === "plugin:event|unlisten") {
        return Promise.resolve(Math.floor(Math.random() * 1000000));
      }
      return origInvoke(cmd, args);
    };
    window.__TAURI_INTERNALS__.invoke.__patched = true;
  }
})();
`;
}
