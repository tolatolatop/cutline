/**
 * Tauri API mock layer for Playwright tests.
 *
 * When testing against the Vite dev server (not the Tauri binary),
 * `window.__TAURI_INTERNALS__` is undefined. We inject this mock
 * via `page.addInitScript()` so that the frontend's `invoke()` calls
 * are routed to deterministic mock data.
 */

export function buildMockScript(): string {
  return `
(function () {
  var assets = [];
  var tasks = [];
  var project = null;

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
    return {
      schemaVersion: "0.1.0",
      project: {
        projectId: makeId(),
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

  function guessType(p) {
    var ext = p.split(".").pop().toLowerCase();
    if (["mp4","mov","avi","mkv","webm","flv","wmv"].indexOf(ext) >= 0) return "video";
    if (["mp3","wav","aac","flac","ogg","wma"].indexOf(ext) >= 0) return "audio";
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
      path: "workspace/assets/" + type + "/" + filePath.split(/[\\\\\\/]/).pop(),
      meta: makeMeta(type),
      tags: [],
      createdAt: now(),
    };
    assets.push(asset);
    project.indexes.assetById[asset.assetId] = assets.length - 1;

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

  var handlers = {
    create_project: function (args) {
      project = makeProject(args.name || "Untitled");
      return project;
    },
    open_project: function () {
      if (!project) project = makeProject("Opened Project");
      return project;
    },
    save_project: function () { return null; },
    get_project: function () { return project; },
    import_assets: function (args) {
      var paths = args.filePaths || [];
      return paths.map(function (p) { return importAsset(p); });
    },
    probe_media: function () { return {}; },
    read_file_base64: function () { return ""; },
    task_enqueue: function () { return makeId(); },
    task_retry: function () { return null; },
    task_cancel: function () { return null; },
    task_list: function () {
      return tasks.map(function (t) {
        return { taskId: t.taskId, kind: t.kind, state: t.state, createdAt: t.createdAt, updatedAt: t.updatedAt, progress: t.progress, error: t.error, retries: t.retries };
      });
    },
  };

  // Dialog plugin handlers
  // Note: the dialog plugin wraps args inside { options: { ... } }
  var dialogHandlers = {
    "plugin:dialog|open": function (args) {
      var opts = (args && args.options) || args || {};
      if (opts.directory) {
        var dir = window.__TAURI_MOCK_DIALOG_DIR__ || null;
        window.__TAURI_MOCK_DIALOG_DIR__ = null;
        return dir;
      }
      if (opts.multiple) {
        var files = window.__TAURI_MOCK_DIALOG_FILES__ || null;
        window.__TAURI_MOCK_DIALOG_FILES__ = null;
        return files;
      }
      var files2 = window.__TAURI_MOCK_DIALOG_FILES__ || null;
      window.__TAURI_MOCK_DIALOG_FILES__ = null;
      return files2;
    },
    "plugin:dialog|save": function () { return null; },
    "plugin:dialog|message": function () { return null; },
    "plugin:dialog|ask": function () { return true; },
    "plugin:dialog|confirm": function () { return true; },
  };

  window.__TAURI_INTERNALS__ = {
    invoke: function (cmd, args, options) {
      // Event plugin
      if (cmd === "plugin:event|listen" || cmd === "plugin:event|unlisten") {
        return Promise.resolve(Math.floor(Math.random() * 1000000));
      }
      // Dialog plugin
      if (dialogHandlers[cmd]) {
        return Promise.resolve(dialogHandlers[cmd](args || {}));
      }
      // App commands
      if (handlers[cmd]) {
        return Promise.resolve(handlers[cmd](args || {}));
      }
      // FS plugin — ignore silently
      if (cmd.indexOf("plugin:fs|") === 0) {
        return Promise.resolve(null);
      }
      // Opener plugin
      if (cmd.indexOf("plugin:opener|") === 0) {
        return Promise.resolve(null);
      }
      console.warn("[tauri-mock] unhandled command:", cmd, args);
      return Promise.resolve(null);
    },
    transformCallback: function (callback) {
      var id = Math.floor(Math.random() * 1000000);
      window["_" + id] = callback;
      return id;
    },
    metadata: {
      currentWebview: { label: "main" },
      currentWindow: { label: "main" },
    },
  };
})();
`;
}
