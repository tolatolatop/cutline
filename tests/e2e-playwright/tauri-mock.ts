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
  var eventListeners = {};

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
    var videoTrackId = makeId();
    var audioTrackId = makeId();
    var textTrackId = makeId();
    return {
      schemaVersion: "0.2",
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
        defaultDraftTrackIds: { video: videoTrackId, audio: audioTrackId, text: textTrackId },
      },
      assets: assets,
      tasks: tasks,
      timeline: {
        timelineId: makeId(),
        timebase: { fps: 24, unit: "seconds" },
        tracks: [
          { trackId: videoTrackId, type: "video", name: "Draft Video", clipIds: [] },
          { trackId: audioTrackId, type: "audio", name: "Draft Audio", clipIds: [] },
          { trackId: textTrackId, type: "text", name: "Notes / Prompts", clipIds: [] },
        ],
        clips: {},
        markers: [],
        durationMs: 0,
      },
      exports: [],
      indexes: { assetById: {}, taskById: {}, clipById: {} },
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

  function emitMockEvent(eventName, payload) {
    var listeners = eventListeners[eventName] || [];
    for (var i = 0; i < listeners.length; i++) {
      var handlerId = listeners[i];
      var fn = window["_" + handlerId];
      if (typeof fn === "function") {
        try { fn({ event: eventName, payload: payload || null }); } catch(e) {}
      }
    }
  }

  function recalcDuration() {
    if (!project) return;
    var maxMs = 0;
    var clipKeys = Object.keys(project.timeline.clips);
    for (var i = 0; i < clipKeys.length; i++) {
      var c = project.timeline.clips[clipKeys[i]];
      var end = c.startMs + c.durationMs;
      if (end > maxMs) maxMs = end;
    }
    project.timeline.durationMs = maxMs;
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

  var providersStore = {};
  var secretsStore = {};

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
    timeline_add_clip: function (args) {
      var clipId = makeId();
      var assetObj = assets.find(function(a) { return a.assetId === args.assetId; });
      var dur = (assetObj && assetObj.meta && assetObj.meta.durationSec) ? Math.round(assetObj.meta.durationSec * 1000) : 1000;
      var clip = { clipId: clipId, assetId: args.assetId, trackId: args.trackId, startMs: args.startMs || 0, durationMs: dur, inMs: 0, outMs: dur };
      project.timeline.clips[clipId] = clip;
      project.indexes.clipById[clipId] = clip;
      var track = project.timeline.tracks.find(function(t) { return t.trackId === args.trackId; });
      if (track) track.clipIds.push(clipId);
      recalcDuration();
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return clip;
    },
    timeline_move_clip: function (args) {
      var clip = project.timeline.clips[args.clipId];
      if (clip && typeof args.newStartMs === "number") {
        clip.startMs = args.newStartMs;
        recalcDuration();
      }
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return null;
    },
    timeline_trim_clip: function (args) {
      var clip = project.timeline.clips[args.clipId];
      if (clip) {
        if (typeof args.newInMs === "number") clip.inMs = args.newInMs;
        if (typeof args.newOutMs === "number") clip.outMs = args.newOutMs;
        clip.durationMs = clip.outMs - clip.inMs;
        recalcDuration();
      }
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return null;
    },
    timeline_remove_clip: function (args) {
      var clip = project.timeline.clips[args.clipId];
      if (clip) {
        delete project.timeline.clips[args.clipId];
        delete project.indexes.clipById[args.clipId];
        project.timeline.tracks.forEach(function(track) {
          var idx = track.clipIds.indexOf(args.clipId);
          if (idx >= 0) track.clipIds.splice(idx, 1);
        });
        recalcDuration();
      }
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return null;
    },
    timeline_reorder_clips: function () { return null; },
    marker_add: function (args) {
      var marker = { markerId: makeId(), tMs: args.tMs || 0, label: args.label || "", promptText: args.promptText || "", createdAt: now() };
      project.timeline.markers.push(marker);
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return marker;
    },
    marker_update: function (args) {
      var m = project.timeline.markers.find(function(mk) { return mk.markerId === args.markerId; });
      if (m) {
        if (args.label !== undefined) m.label = args.label;
        if (args.promptText !== undefined) m.promptText = args.promptText;
      }
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return null;
    },
    marker_remove: function (args) {
      project.timeline.markers = project.timeline.markers.filter(function(mk) { return mk.markerId !== args.markerId; });
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return null;
    },
    providers_list: function () {
      var list = [];
      var keys = Object.keys(providersStore);
      for (var i = 0; i < keys.length; i++) {
        var name = keys[i];
        var cfg = providersStore[name];
        list.push({
          name: name,
          displayName: cfg.displayName || name,
          authKind: cfg.auth ? cfg.auth.kind : "api_key",
          profiles: Object.keys(cfg.profiles || {}),
        });
      }
      return list;
    },
    providers_get: function (args) {
      var cfg = providersStore[args.name];
      if (!cfg) throw new Error("provider_not_found: " + args.name);
      return cfg;
    },
    providers_upsert: function (args) {
      providersStore[args.name] = args.config;
      return null;
    },
    providers_delete: function (args) {
      delete providersStore[args.name];
      return null;
    },
    secrets_set: function (args) {
      secretsStore[args.credentialRef] = true;
      return null;
    },
    secrets_exists: function (args) {
      return !!secretsStore[args.credentialRef];
    },
    secrets_delete: function (args) {
      delete secretsStore[args.credentialRef];
      return null;
    },
    providers_test: function (args) {
      var cfg = providersStore[args.providerName];
      if (!cfg) return { ok: false, error: "provider_not_found" };
      var prof = cfg.profiles && cfg.profiles[args.profileName];
      if (!prof) return { ok: false, error: "profile_not_found" };
      if (!secretsStore[prof.credentialRef]) return { ok: false, error: "missing_credentials" };
      return { ok: true, latencyMs: 42 };
    },
    update_generation_settings: function (args) {
      if (project) {
        if (!project.project.settings.generation) {
          project.project.settings.generation = {};
        }
        project.project.settings.generation.videoProvider = args.videoProvider || null;
        project.project.settings.generation.videoProfile = args.videoProfile || null;
      }
      setTimeout(function() { emitMockEvent("project:updated", {}); }, 50);
      return null;
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
      if (cmd === "plugin:event|listen") {
        var evtName = (args && args.event) || "";
        var handlerId = args && args.handler;
        if (evtName && handlerId !== undefined) {
          if (!eventListeners[evtName]) eventListeners[evtName] = [];
          eventListeners[evtName].push(handlerId);
        }
        return Promise.resolve(Math.floor(Math.random() * 1000000));
      }
      if (cmd === "plugin:event|unlisten") {
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
