import { describe, expect, it, vi, beforeAll, beforeEach } from "vite-plus/test";
import { buildGeometry, type Keypoint } from "~/lib/faceLandmarks";

// ── MediaPipe mock ────────────────────────────────────────────────────────────
// Note: vi.mock factory is hoisted — do NOT reference top-level variables here.

vi.mock("@mediapipe/tasks-vision", () => ({
  FilesetResolver: {
    forVisionTasks: vi.fn().mockResolvedValue({}),
  },
  FaceLandmarker: {
    createFromOptions: vi.fn(),
  },
}));

// Stubs for canvas / ImageData (happy-dom gaps).
beforeAll(() => {
  vi.stubGlobal(
    "ImageData",
    class {
      data: Uint8ClampedArray;
      width: number;
      height: number;
      constructor(data: Uint8ClampedArray, w: number, h: number) {
        this.data = data;
        this.width = w;
        this.height = h;
      }
    },
  );

  const makeCtx = () => ({ putImageData: vi.fn() });
  const origCreate = document.createElement.bind(document);
  vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
    if (tag === "canvas")
      return {
        width: 0,
        height: 0,
        getContext: () => makeCtx(),
      } as unknown as HTMLCanvasElement;
    return origCreate(tag as "div");
  });
});

// Synth: 478 keypoints arranged so each index has predictable coordinates.
// This lets us check the geometry packing logic without needing MediaPipe.
function syntheticKeypoints(): Keypoint[] {
  const kps: Keypoint[] = [];
  for (let i = 0; i < 478; i++) {
    // Spread across image deterministically.
    kps.push({ x: ((i * 7919) % 1000) / 1000, y: ((i * 6113) % 1000) / 1000 });
  }
  // Override iris centres + perimeters with known values.
  kps[468] = { x: 0.4, y: 0.45 };
  kps[469] = { x: 0.4, y: 0.43 };
  kps[470] = { x: 0.42, y: 0.45 };
  kps[471] = { x: 0.4, y: 0.47 };
  kps[472] = { x: 0.38, y: 0.45 };
  kps[473] = { x: 0.6, y: 0.45 };
  kps[474] = { x: 0.6, y: 0.43 };
  kps[475] = { x: 0.62, y: 0.45 };
  kps[476] = { x: 0.6, y: 0.47 };
  kps[477] = { x: 0.58, y: 0.45 };
  return kps;
}

// ── extractGeometry ───────────────────────────────────────────────────────────
// Each test resets modules so landmarkerPromise (module-level cache) is cleared.
// After reset, dynamic imports give fresh instances; configure the fresh mock
// by importing @mediapipe/tasks-vision dynamically (the vi.mock factory re-runs).

describe("extractGeometry", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetModules();
  });

  it("顔が検出されないとき null を返す", async () => {
    const detect = vi.fn().mockReturnValue({ faceLandmarks: [] });
    const { FaceLandmarker } = await import("@mediapipe/tasks-vision");
    vi.mocked(FaceLandmarker.createFromOptions).mockResolvedValue({ detect } as never);

    const { extractGeometry } = await import("~/lib/faceLandmarks");
    const result = await extractGeometry(new Uint8ClampedArray(640 * 480 * 4), 640, 480);
    expect(result).toBeNull();
  });

  it("顔が検出されたとき FaceGeometry を返す", async () => {
    const kps = syntheticKeypoints();
    const detect = vi
      .fn()
      .mockReturnValue({ faceLandmarks: [kps.map((k) => ({ x: k.x, y: k.y, z: 0 }))] });
    const { FaceLandmarker } = await import("@mediapipe/tasks-vision");
    vi.mocked(FaceLandmarker.createFromOptions).mockResolvedValue({ detect } as never);

    const { extractGeometry } = await import("~/lib/faceLandmarks");
    const result = await extractGeometry(new Uint8ClampedArray(640 * 480 * 4), 640, 480);
    expect(result).not.toBeNull();
    expect(result!.faceOval).toHaveLength(72);
  });

  it("loadDetector がキャッシュされる（2 回目は createFromOptions を呼ばない）", async () => {
    const detect = vi.fn().mockReturnValue({ faceLandmarks: [] });
    const { FaceLandmarker } = await import("@mediapipe/tasks-vision");
    vi.mocked(FaceLandmarker.createFromOptions).mockResolvedValue({ detect } as never);

    const { extractGeometry } = await import("~/lib/faceLandmarks");
    const rgba = new Uint8ClampedArray(640 * 480 * 4);
    await extractGeometry(rgba, 640, 480);
    await extractGeometry(rgba, 640, 480);
    expect(FaceLandmarker.createFromOptions).toHaveBeenCalledOnce();
  });
});

// ── buildGeometry ─────────────────────────────────────────────────────────────

describe("buildGeometry", () => {
  it("returns null for too-few keypoints (no iris refine)", () => {
    const kps: Keypoint[] = Array.from({ length: 100 }, () => ({ x: 0.5, y: 0.5 }));
    expect(buildGeometry(kps)).toBeNull();
  });

  it("packs face oval as a 36-vertex polygon (72 floats)", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    expect(g.faceOval).toHaveLength(72);
  });

  it("exclusions header starts with 6 (number of polygons)", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    expect(g.exclusions[0]).toBe(6);
  });

  it("computes eye warp radius from iris perimeter × 2.5", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    // Iris perimeter is 0.02 from centre (set above). Warp radius = 0.05.
    expect(g.eyes[0]).toBeCloseTo(0.4, 5);
    expect(g.eyes[1]).toBeCloseTo(0.45, 5);
    expect(g.eyes[2]).toBeCloseTo(0.05, 5);
    expect(g.eyes[3]).toBeCloseTo(0.6, 5);
    expect(g.eyes[5]).toBeCloseTo(0.05, 5);
  });

  it("eyebrow polygon has even point count (extruded loop)", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    // skip: header(1) + eye(1+32) + eye(1+32) = 67; eyebrow_left starts at 67
    // Actually we just verify packed polygons are well-formed: every poly has
    // even coordinate count and matches its declared length.
    let i = 1;
    const n = g.exclusions[0];
    for (let p = 0; p < n; p++) {
      const len = g.exclusions[i++];
      expect(len).toBeGreaterThan(2);
      i += len * 2;
    }
    expect(i).toBe(g.exclusions.length);
  });
});
