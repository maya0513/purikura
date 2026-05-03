import { describe, expect, it, vi, beforeAll, beforeEach } from "vite-plus/test";
import {
  clamp01,
  buildMakeupJson,
  computeEyesGeom,
  processPhoto,
  renderFinal,
  initWasm,
} from "~/lib/imageProcessor";
import type { BeautyParams, MakeupParams, ToneParams, BackgroundParams } from "~/state/types";

// Mock face detection and segmentation (heavy deps not needed in unit tests).
vi.mock("~/lib/faceLandmarks", () => ({
  extractGeometry: vi.fn().mockResolvedValue(null),
}));
vi.mock("~/lib/segmentation", () => ({
  extractSegmentationMask: vi.fn().mockResolvedValue(null),
}));

// Stub DOM to avoid happy-dom limitations with Image.onload / canvas / ImageData.
beforeAll(() => {
  // Image: fire onload asynchronously for any src.
  vi.stubGlobal(
    "Image",
    class {
      onload: (() => void) | null = null;
      onerror: ((e: unknown) => void) | null = null;
      width = 1;
      height = 1;
      set src(_: string) {
        Promise.resolve().then(() => this.onload?.());
      }
    },
  );

  // ImageData: minimal stub so new ImageData(...) doesn't throw.
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

  // Canvas: return a minimal 2D context with no-op methods and blank pixel data.
  const makeCtx = () => ({
    drawImage: vi.fn(),
    putImageData: vi.fn(),
    getImageData: () => ({ data: new Uint8ClampedArray(640 * 480 * 4) }),
    fillText: vi.fn(),
    textAlign: "" as CanvasTextAlign,
    textBaseline: "" as CanvasTextBaseline,
    font: "",
  });
  const makeCanvas = () =>
    ({
      width: 0,
      height: 0,
      getContext: () => makeCtx(),
      toDataURL: () => "data:image/png;base64,stub",
    }) as unknown as HTMLCanvasElement;

  const origCreate = document.createElement.bind(document);
  vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
    if (tag === "canvas") return makeCanvas();
    return origCreate(tag as "div");
  });
});

// Default params helpers.
function beautyOff(): BeautyParams {
  return {
    skin: false,
    blemish: false,
    eyes: false,
    slim: false,
    whiten: false,
    eyeSparkle: false,
    strength: 0.8,
    blemishStrength: 0.9,
    eyesStrength: 0.5,
    slimStrength: 0.3,
    whitenStrength: 0.75,
    eyeSparkleStrength: 0.65,
  };
}
function makeupOff(): MakeupParams {
  return {
    lipEnabled: false,
    lipColor: "#ff0000",
    lipStrength: 0.5,
    eyeShadowEnabled: false,
    eyeShadowColor: "#0000ff",
    eyeShadowStrength: 0.3,
    blushEnabled: false,
    blushColor: "#ff8888",
    blushStrength: 0.3,
  };
}
function toneOff(): ToneParams {
  return {
    lutPreset: "none",
    overlayEnabled: false,
    overlayColor: "#ffffff",
    overlayStrength: 0.1,
    overlayBlendMode: "normal",
    vignetteStrength: 0,
  };
}
function bgOff(): BackgroundParams {
  return { mode: "none", blurRadius: 15, solidColor: "#ffffff", imageDataUrl: null };
}

// ── clamp01 ───────────────────────────────────────────────────────────────────

describe("clamp01", () => {
  it("0 未満は 0 を返す", () => {
    expect(clamp01(-5)).toBe(0);
  });

  it("1 超は 1 を返す", () => {
    expect(clamp01(1.5)).toBe(1);
  });

  it("中間値はそのまま返す", () => {
    expect(clamp01(0.42)).toBeCloseTo(0.42);
  });

  it("0 と 1 の境界値はそのまま返す", () => {
    expect(clamp01(0)).toBe(0);
    expect(clamp01(1)).toBe(1);
  });
});

// ── buildMakeupJson ───────────────────────────────────────────────────────────

describe("buildMakeupJson", () => {
  it("lip_enabled フィールドが含まれる", () => {
    const json = buildMakeupJson({ ...makeupOff(), lipEnabled: true });
    expect(json).toHaveProperty("lip_enabled", true);
  });

  it("hex カラーが rgb 値に変換される", () => {
    const json = buildMakeupJson({
      ...makeupOff(),
      lipColor: "#ff0000",
      lipEnabled: true,
    }) as Record<string, unknown>;
    expect(json["lip_r"]).toBe(255);
    expect(json["lip_g"]).toBe(0);
    expect(json["lip_b"]).toBe(0);
  });

  it("strength が clamp01 を通る", () => {
    const json = buildMakeupJson({ ...makeupOff(), lipStrength: 1.5 }) as Record<string, unknown>;
    expect(json["lip_strength"]).toBe(1);
  });

  it("eye_shadow フィールドが含まれる", () => {
    const json = buildMakeupJson(makeupOff()) as Record<string, unknown>;
    expect(json).toHaveProperty("eye_shadow_enabled", false);
  });

  it("blush フィールドが含まれる", () => {
    const json = buildMakeupJson(makeupOff()) as Record<string, unknown>;
    expect(json).toHaveProperty("blush_enabled", false);
  });
});

// ── computeEyesGeom ───────────────────────────────────────────────────────────

describe("computeEyesGeom", () => {
  it("strength ≤ 1 では元の配列をそのまま返す", () => {
    const eyes = new Float32Array([0.4, 0.45, 0.05, 0.6, 0.45, 0.05]);
    const result = computeEyesGeom(eyes, 0.8);
    expect(result).toBe(eyes); // 同じ参照
  });

  it("strength > 1 では半径要素が radiusBoost 倍になる", () => {
    const eyes = new Float32Array([0.4, 0.45, 0.04, 0.6, 0.45, 0.04]);
    const result = computeEyesGeom(eyes, 1.5);
    // radiusBoost = 1 + (1.5 - 1) = 1.5
    expect(result[2]).toBeCloseTo(0.04 * 1.5);
    expect(result[5]).toBeCloseTo(0.04 * 1.5);
  });

  it("strength <= 0 でも元の配列をそのまま返す", () => {
    const eyes = new Float32Array([0.4, 0.45, 0.05]);
    const result = computeEyesGeom(eyes, 0);
    expect(result).toBe(eyes);
  });
});

// ── processPhoto ─────────────────────────────────────────────────────────────

describe("processPhoto", () => {
  beforeEach(async () => {
    await initWasm();
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    vi.mocked(wasm.apply_filter).mockClear();
    vi.mocked(wasm.remove_blemish).mockClear();
    vi.mocked(wasm.apply_beauty).mockClear();
    vi.mocked(wasm.enlarge_eyes).mockClear();
    vi.mocked(wasm.slim_face).mockClear();
    vi.mocked(wasm.whiten_skin).mockClear();
    vi.mocked(wasm.apply_eye_sparkle).mockClear();
    vi.mocked(wasm.apply_makeup).mockClear();
    vi.mocked(wasm.apply_color_overlay).mockClear();
    vi.mocked(wasm.apply_lut3d).mockClear();
    vi.mocked(wasm.process_background).mockClear();
  });

  it("全フラグ OFF・filter=none では WASM 関数が呼ばれない", async () => {
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      beautyOff(),
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.apply_filter).not.toHaveBeenCalled();
    expect(wasm.remove_blemish).not.toHaveBeenCalled();
  });

  it("filter != 'none' で apply_filter が呼ばれる", async () => {
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "warm",
      beautyOff(),
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.apply_filter).toHaveBeenCalledWith(
      expect.any(Uint8Array),
      expect.any(Number),
      expect.any(Number),
      "warm",
    );
  });

  it("tone.lutPreset != 'none' で apply_lut3d が呼ばれる", async () => {
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      beautyOff(),
      makeupOff(),
      { ...toneOff(), lutPreset: "pop" },
      bgOff(),
    );
    expect(wasm.apply_lut3d).toHaveBeenCalledWith(
      expect.any(Uint8Array),
      expect.any(Number),
      expect.any(Number),
      "pop",
    );
  });

  it("tone.overlayEnabled=true で apply_color_overlay が呼ばれる", async () => {
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      beautyOff(),
      makeupOff(),
      { ...toneOff(), overlayEnabled: true, overlayColor: "#ff0000" },
      bgOff(),
    );
    expect(wasm.apply_color_overlay).toHaveBeenCalled();
  });

  it("vignetteStrength > 0 のみで apply_color_overlay が呼ばれる", async () => {
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      beautyOff(),
      makeupOff(),
      { ...toneOff(), overlayEnabled: false, vignetteStrength: 0.5 },
      bgOff(),
    );
    expect(wasm.apply_color_overlay).toHaveBeenCalled();
  });

  it("beauty.blemish=true かつ顔検出成功で remove_blemish が呼ばれる", async () => {
    const { extractGeometry } = await import("~/lib/faceLandmarks");
    vi.mocked(extractGeometry).mockResolvedValueOnce({
      faceOval: new Float32Array([0.3, 0.3, 0.7, 0.3, 0.7, 0.7, 0.3, 0.7]),
      exclusions: new Float32Array([0]),
      eyes: new Float32Array([0.4, 0.45, 0.05, 0.6, 0.45, 0.05]),
      lipsOuter: new Float32Array([0.4, 0.7, 0.6, 0.7]),
      leftEye: new Float32Array([0.35, 0.45, 0.45, 0.45]),
      rightEye: new Float32Array([0.55, 0.45, 0.65, 0.45]),
      cheeks: new Float32Array([0.25, 0.5, 0.75, 0.5]),
    });
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      { ...beautyOff(), blemish: true },
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.remove_blemish).toHaveBeenCalled();
  });

  // Helper: mock extractGeometry to return a minimal valid FaceGeometry.
  async function withGeometry() {
    const { extractGeometry } = await import("~/lib/faceLandmarks");
    const geom = {
      faceOval: new Float32Array([0.3, 0.3, 0.7, 0.3, 0.7, 0.7, 0.3, 0.7]),
      exclusions: new Float32Array([0]),
      eyes: new Float32Array([0.4, 0.45, 0.05, 0.6, 0.45, 0.05]),
      lipsOuter: new Float32Array([0.4, 0.7, 0.6, 0.7]),
      leftEye: new Float32Array([0.35, 0.45, 0.45, 0.45]),
      rightEye: new Float32Array([0.55, 0.45, 0.65, 0.45]),
      cheeks: new Float32Array([0.25, 0.5, 0.75, 0.5]),
    };
    vi.mocked(extractGeometry).mockResolvedValueOnce(geom);
    return geom;
  }

  it("beauty.skin=true かつ顔検出成功で apply_beauty が呼ばれる", async () => {
    await withGeometry();
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      { ...beautyOff(), skin: true },
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.apply_beauty).toHaveBeenCalled();
  });

  it("beauty.eyes=true かつ顔検出成功で enlarge_eyes が呼ばれる", async () => {
    await withGeometry();
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      { ...beautyOff(), eyes: true },
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.enlarge_eyes).toHaveBeenCalled();
  });

  it("beauty.slim=true かつ顔検出成功で slim_face が呼ばれる", async () => {
    await withGeometry();
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      { ...beautyOff(), slim: true, slimStrength: 0.5 },
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.slim_face).toHaveBeenCalled();
  });

  it("beauty.whiten=true かつ顔検出成功で whiten_skin が呼ばれる", async () => {
    await withGeometry();
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      { ...beautyOff(), whiten: true },
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.whiten_skin).toHaveBeenCalled();
  });

  it("beauty.eyeSparkle=true かつ顔検出成功で apply_eye_sparkle が呼ばれる", async () => {
    await withGeometry();
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      { ...beautyOff(), eyeSparkle: true },
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.apply_eye_sparkle).toHaveBeenCalled();
  });

  it("beauty.whiten=false では whiten_skin が呼ばれない", async () => {
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      beautyOff(),
      makeupOff(),
      toneOff(),
      bgOff(),
    );
    expect(wasm.whiten_skin).not.toHaveBeenCalled();
  });

  it("makeup.lipEnabled=true かつ顔検出成功で apply_makeup が呼ばれる", async () => {
    await withGeometry();
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto(
      "data:image/png;base64,test",
      "none",
      beautyOff(),
      { ...makeupOff(), lipEnabled: true },
      toneOff(),
      bgOff(),
    );
    expect(wasm.apply_makeup).toHaveBeenCalled();
  });

  it("background.mode='blur' で process_background が呼ばれる", async () => {
    const { extractSegmentationMask } = await import("~/lib/segmentation");
    vi.mocked(extractSegmentationMask).mockResolvedValueOnce(new Float32Array(640 * 480).fill(1));
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto("data:image/png;base64,test", "none", beautyOff(), makeupOff(), toneOff(), {
      mode: "blur",
      blurRadius: 15,
      solidColor: "#ffffff",
      imageDataUrl: null,
    });
    expect(wasm.process_background).toHaveBeenCalled();
  });

  it("background.mode='solid' で process_background が呼ばれる", async () => {
    const wasm = await import("~/wasm/pkg/purikura_wasm");
    await processPhoto("data:image/png;base64,test", "none", beautyOff(), makeupOff(), toneOff(), {
      mode: "solid",
      blurRadius: 15,
      solidColor: "#ff0000",
      imageDataUrl: null,
    });
    expect(wasm.process_background).toHaveBeenCalled();
  });
});

// ── renderFinal ──────────────────────────────────────────────────────────────

describe("renderFinal", () => {
  it("items が空でも文字列を返す", async () => {
    const result = await renderFinal("data:image/png;base64,test", []);
    expect(typeof result).toBe("string");
  });

  it("items に emoji があっても文字列を返す", async () => {
    const result = await renderFinal("data:image/png;base64,test", [
      { id: "1", emoji: "✨", x: 0.5, y: 0.5 },
    ]);
    expect(typeof result).toBe("string");
  });
});
