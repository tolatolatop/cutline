import { describe, it, expect, beforeEach } from "vitest";
import {
  useTimelineViewStore,
  msToPixels,
  pixelsToMs,
  formatMs,
} from "./timelineViewStore";
import type { Clip } from "../models/project";

function store() {
  return useTimelineViewStore.getState();
}

function makeClip(id: string, startMs: number, durationMs: number, trackId = "trk_v"): Clip {
  return {
    clipId: id,
    assetId: `asset_${id}`,
    trackId,
    startMs,
    durationMs,
    inMs: 0,
    outMs: durationMs,
  };
}

function makeClipsMap(clips: Clip[]): Record<string, Clip> {
  const map: Record<string, Clip> = {};
  for (const c of clips) map[c.clipId] = c;
  return map;
}

beforeEach(() => {
  useTimelineViewStore.setState({
    playheadMs: 0,
    isPlaying: false,
    zoomLevel: 100,
    scrollLeftMs: 0,
    selectedClipIds: new Set(),
  });
});

// ============================================================
// Selection: selectClip (single)
// ============================================================

describe("selectClip", () => {
  it("selects a single clip", () => {
    store().selectClip("c1");
    expect(store().selectedClipIds).toEqual(new Set(["c1"]));
  });

  it("replaces previous selection", () => {
    store().selectClip("c1");
    store().selectClip("c2");
    expect(store().selectedClipIds).toEqual(new Set(["c2"]));
  });

  it("null clears selection", () => {
    store().selectClip("c1");
    store().selectClip(null);
    expect(store().selectedClipIds.size).toBe(0);
  });
});

// ============================================================
// Selection: toggleClip (Ctrl+Click)
// ============================================================

describe("toggleClip", () => {
  it("adds unselected clip", () => {
    store().selectClip("c1");
    store().toggleClip("c2");
    expect(store().selectedClipIds).toEqual(new Set(["c1", "c2"]));
  });

  it("removes already-selected clip", () => {
    store().selectClip("c1");
    store().toggleClip("c2");
    store().toggleClip("c1");
    expect(store().selectedClipIds).toEqual(new Set(["c2"]));
  });

  it("toggles same clip on and off", () => {
    store().toggleClip("c1");
    expect(store().selectedClipIds.has("c1")).toBe(true);
    store().toggleClip("c1");
    expect(store().selectedClipIds.has("c1")).toBe(false);
  });
});

// ============================================================
// Selection: selectClips (batch)
// ============================================================

describe("selectClips", () => {
  it("replaces selection with given ids", () => {
    store().selectClip("old");
    store().selectClips(["a", "b", "c"]);
    expect(store().selectedClipIds).toEqual(new Set(["a", "b", "c"]));
  });

  it("empty array clears selection", () => {
    store().selectClip("c1");
    store().selectClips([]);
    expect(store().selectedClipIds.size).toBe(0);
  });
});

// ============================================================
// Selection: addClips (Ctrl+marquee)
// ============================================================

describe("addClips", () => {
  it("appends to existing selection", () => {
    store().selectClip("c1");
    store().addClips(["c2", "c3"]);
    expect(store().selectedClipIds).toEqual(new Set(["c1", "c2", "c3"]));
  });

  it("does not duplicate existing ids", () => {
    store().selectClips(["c1", "c2"]);
    store().addClips(["c2", "c3"]);
    expect(store().selectedClipIds).toEqual(new Set(["c1", "c2", "c3"]));
  });
});

// ============================================================
// Selection: clearSelection
// ============================================================

describe("clearSelection", () => {
  it("clears all selected clips", () => {
    store().selectClips(["a", "b", "c"]);
    store().clearSelection();
    expect(store().selectedClipIds.size).toBe(0);
  });
});

// ============================================================
// Selection: selectRange (time-based)
// ============================================================

describe("selectRange", () => {
  const clips = makeClipsMap([
    makeClip("v1", 0, 5000, "trk_v"),       // 0..5000
    makeClip("v2", 5000, 3000, "trk_v"),     // 5000..8000
    makeClip("a1", 1000, 4000, "trk_a"),     // 1000..5000
    makeClip("t1", 6000, 2000, "trk_t"),     // 6000..8000
    makeClip("v3", 10000, 2000, "trk_v"),    // 10000..12000
  ]);

  it("selects clips overlapping the range", () => {
    store().selectRange(500, 4500, clips);
    const sel = store().selectedClipIds;
    expect(sel.has("v1")).toBe(true);  // 0..5000 overlaps 500..4500
    expect(sel.has("a1")).toBe(true);  // 1000..5000 overlaps 500..4500
    expect(sel.has("v2")).toBe(false); // 5000..8000 does not overlap 500..4500
    expect(sel.has("t1")).toBe(false);
    expect(sel.has("v3")).toBe(false);
  });

  it("selects across all track types", () => {
    store().selectRange(0, 9000, clips);
    const sel = store().selectedClipIds;
    expect(sel.has("v1")).toBe(true);
    expect(sel.has("v2")).toBe(true);
    expect(sel.has("a1")).toBe(true);
    expect(sel.has("t1")).toBe(true);
    expect(sel.has("v3")).toBe(false); // starts at 10000
  });

  it("handles reversed range (endMs < startMs)", () => {
    store().selectRange(8000, 500, clips);
    const sel = store().selectedClipIds;
    expect(sel.has("v1")).toBe(true);
    expect(sel.has("v2")).toBe(true);
    expect(sel.has("a1")).toBe(true);
    expect(sel.has("t1")).toBe(true);
  });

  it("empty range selects nothing", () => {
    store().selectRange(8500, 9500, clips);
    expect(store().selectedClipIds.size).toBe(0);
  });

  it("exact boundary: range ending at clip start does not select", () => {
    store().selectRange(0, 5000, clips);
    const sel = store().selectedClipIds;
    expect(sel.has("v1")).toBe(true);
    expect(sel.has("v2")).toBe(false); // starts exactly at 5000, range hi=5000, 5000 < 5000 is false
  });

  it("range covering entire timeline", () => {
    store().selectRange(0, 15000, clips);
    expect(store().selectedClipIds.size).toBe(5);
  });

  it("empty clips map selects nothing", () => {
    store().selectRange(0, 10000, {});
    expect(store().selectedClipIds.size).toBe(0);
  });
});

// ============================================================
// Utility functions
// ============================================================

describe("msToPixels", () => {
  it("converts at zoom 100", () => {
    expect(msToPixels(1000, 100)).toBe(100);
  });

  it("converts at zoom 50", () => {
    expect(msToPixels(1000, 50)).toBe(50);
  });

  it("converts at zoom 200", () => {
    expect(msToPixels(1000, 200)).toBe(200);
  });

  it("zero ms returns zero", () => {
    expect(msToPixels(0, 100)).toBe(0);
  });
});

describe("pixelsToMs", () => {
  it("converts at zoom 100", () => {
    expect(pixelsToMs(100, 100)).toBe(1000);
  });

  it("roundtrip with msToPixels", () => {
    const ms = 3456;
    const zoom = 100;
    expect(pixelsToMs(msToPixels(ms, zoom), zoom)).toBeCloseTo(ms);
  });
});

describe("formatMs", () => {
  it("formats zero", () => {
    expect(formatMs(0)).toBe("00:00.00");
  });

  it("formats 1 second", () => {
    expect(formatMs(1000)).toBe("00:01.00");
  });

  it("formats 1 minute 30 seconds", () => {
    expect(formatMs(90000)).toBe("01:30.00");
  });

  it("formats with centiseconds", () => {
    expect(formatMs(1500)).toBe("00:01.50");
  });

  it("formats fractional ms", () => {
    expect(formatMs(1234)).toBe("00:01.23");
  });
});

// ============================================================
// Mixed multi-select workflow simulation
// ============================================================

describe("multi-select workflow", () => {
  const clips = makeClipsMap([
    makeClip("v1", 0, 5000, "trk_v"),
    makeClip("a1", 0, 5000, "trk_a"),
    makeClip("t1", 0, 5000, "trk_t"),
    makeClip("v2", 6000, 3000, "trk_v"),
  ]);

  it("click selects one, then Ctrl+click adds another", () => {
    store().selectClip("v1");
    expect(store().selectedClipIds.size).toBe(1);

    store().toggleClip("a1");
    expect(store().selectedClipIds).toEqual(new Set(["v1", "a1"]));
  });

  it("Ctrl+click on selected item deselects it", () => {
    store().selectClips(["v1", "a1", "t1"]);
    store().toggleClip("a1");
    expect(store().selectedClipIds).toEqual(new Set(["v1", "t1"]));
  });

  it("range select then Ctrl+click to remove one", () => {
    store().selectRange(0, 6000, clips);
    expect(store().selectedClipIds.size).toBe(3); // v1, a1, t1

    store().toggleClip("t1");
    expect(store().selectedClipIds).toEqual(new Set(["v1", "a1"]));
  });

  it("addClips extends range selection", () => {
    store().selectRange(0, 6000, clips);
    store().addClips(["v2"]);
    expect(store().selectedClipIds).toEqual(new Set(["v1", "a1", "t1", "v2"]));
  });

  it("Escape clears everything", () => {
    store().selectClips(["v1", "a1", "t1", "v2"]);
    store().clearSelection();
    expect(store().selectedClipIds.size).toBe(0);
  });
});
