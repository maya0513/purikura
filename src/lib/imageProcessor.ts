import type {
  FilterName,
  BeautyParams,
  MakeupParams,
  ToneParams,
  BackgroundParams,
  DekoItem,
} from "~/state/types";
import { PHOTO_WIDTH, PHOTO_HEIGHT, hexToRgb } from "~/state/types";
import { extractGeometry, type FaceGeometry } from "~/lib/faceLandmarks";
import { extractSegmentationMask } from "~/lib/segmentation";

type WasmModule = {
  // Existing sync
  apply_filter: (pixels: Uint8Array, w: number, h: number, filter: string) => Uint8Array;
  build_skin_mask: (
    pixels: Uint8Array,
    w: number,
    h: number,
    faceOval: Float32Array,
    exclusionsPacked: Float32Array,
  ) => Uint8Array;
  apply_beauty: (
    pixels: Uint8Array,
    w: number,
    h: number,
    mask: Uint8Array,
    strength: number,
  ) => Uint8Array;
  remove_blemish: (
    pixels: Uint8Array,
    w: number,
    h: number,
    mask: Uint8Array,
    strength: number,
  ) => Uint8Array;
  enlarge_eyes: (
    pixels: Uint8Array,
    w: number,
    h: number,
    eyes: Float32Array,
    strength: number,
  ) => Uint8Array;
  // New sync
  slim_face: (
    pixels: Uint8Array,
    w: number,
    h: number,
    faceOval: Float32Array,
    strength: number,
  ) => Uint8Array;
  whiten_skin: (
    pixels: Uint8Array,
    w: number,
    h: number,
    mask: Uint8Array,
    strength: number,
  ) => Uint8Array;
  apply_eye_sparkle: (
    pixels: Uint8Array,
    w: number,
    h: number,
    eyes: Float32Array,
    leftEye: Float32Array,
    rightEye: Float32Array,
    strength: number,
  ) => Uint8Array;
  apply_makeup: (
    pixels: Uint8Array,
    w: number,
    h: number,
    lipsOuter: Float32Array,
    leftEye: Float32Array,
    rightEye: Float32Array,
    cheeks: Float32Array,
    paramsJson: string,
  ) => Uint8Array;
  apply_color_overlay: (
    pixels: Uint8Array,
    w: number,
    h: number,
    r: number,
    g: number,
    b: number,
    alpha: number,
    blendMode: string,
    vignette: number,
  ) => Uint8Array;
  // New async (GPU)
  init_gpu: () => Promise<void>;
  apply_lut3d: (pixels: Uint8Array, w: number, h: number, preset: string) => Promise<Uint8Array>;
  process_background: (
    pixels: Uint8Array,
    w: number,
    h: number,
    segMask: Float32Array,
    mode: string,
    r: number,
    g: number,
    b: number,
    replPixels: Uint8Array,
    blurRadius: number,
  ) => Promise<Uint8Array>;
};

let wasm: WasmModule | null = null;

export async function initWasm(): Promise<void> {
  if (wasm) return;
  const mod = await import("~/wasm/pkg/purikura_wasm");
  await mod.default();
  wasm = mod as unknown as WasmModule;
}

export async function initGpu(): Promise<void> {
  if (!wasm) throw new Error("WASM not initialized");
  try {
    await wasm.init_gpu();
  } catch {
    // GPU unavailable — all GPU operations will fall back to CPU silently.
  }
}

const rgbaCache = new Map<string, Uint8ClampedArray>();

function decodeToRgba(src: string): Promise<Uint8ClampedArray> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const c = document.createElement("canvas");
      c.width = PHOTO_WIDTH;
      c.height = PHOTO_HEIGHT;
      c.getContext("2d")!.drawImage(img, 0, 0, PHOTO_WIDTH, PHOTO_HEIGHT);
      resolve(c.getContext("2d")!.getImageData(0, 0, PHOTO_WIDTH, PHOTO_HEIGHT).data);
    };
    img.onerror = reject;
    img.src = src;
  });
}

async function loadRgba(src: string): Promise<Uint8ClampedArray> {
  if (!src.startsWith("data:")) {
    const cached = rgbaCache.get(src);
    if (cached) return cached;
    const data = await decodeToRgba(src);
    rgbaCache.set(src, data);
    return data;
  }
  return decodeToRgba(src);
}

function toDataUrl(pixels: Uint8Array | Uint8ClampedArray): string {
  const c = document.createElement("canvas");
  c.width = PHOTO_WIDTH;
  c.height = PHOTO_HEIGHT;
  c.getContext("2d")!.putImageData(
    new ImageData(new Uint8ClampedArray(pixels), PHOTO_WIDTH, PHOTO_HEIGHT),
    0,
    0,
  );
  return c.toDataURL("image/png");
}

export function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

export function buildMakeupJson(params: MakeupParams): object {
  const [lr, lg, lb] = hexToRgb(params.lipColor);
  const [er, eg, eb] = hexToRgb(params.eyeShadowColor);
  const [br, bg, bb] = hexToRgb(params.blushColor);
  return {
    lip_enabled: params.lipEnabled,
    lip_r: lr,
    lip_g: lg,
    lip_b: lb,
    lip_strength: clamp01(params.lipStrength),
    eye_shadow_enabled: params.eyeShadowEnabled,
    eye_shadow_r: er,
    eye_shadow_g: eg,
    eye_shadow_b: eb,
    eye_shadow_strength: clamp01(params.eyeShadowStrength),
    blush_enabled: params.blushEnabled,
    blush_r: br,
    blush_g: bg,
    blush_b: bb,
    blush_strength: clamp01(params.blushStrength),
  };
}

export function computeEyesGeom(eyes: Float32Array, eyesStrength: number): Float32Array {
  const raw = Math.max(0, eyesStrength);
  const radiusBoost = 1 + Math.max(raw - 1, 0);
  if (radiusBoost === 1) return eyes;
  const boosted = new Float32Array(eyes);
  for (let i = 2; i < boosted.length; i += 3) boosted[i] *= radiusBoost;
  return boosted;
}

export async function processPhoto(
  dataUrl: string,
  filter: FilterName,
  beauty: BeautyParams,
  makeup: MakeupParams,
  tone: ToneParams,
  background: BackgroundParams,
): Promise<string> {
  if (!wasm) throw new Error("WASM not initialized");

  let rgba = await loadRgba(dataUrl);

  const wantsFace =
    beauty.skin ||
    beauty.blemish ||
    beauty.eyes ||
    beauty.slim ||
    beauty.whiten ||
    beauty.eyeSparkle ||
    makeup.lipEnabled ||
    makeup.eyeShadowEnabled ||
    makeup.blushEnabled;
  const wantsBackground = background.mode !== "none";

  // --- Parallel: face detection + segmentation (when needed) ---
  let geometry: FaceGeometry | null = null;
  let segMask: Float32Array | null = null;

  if (wantsFace || wantsBackground) {
    const [geomResult, maskResult] = await Promise.all([
      wantsFace
        ? extractGeometry(rgba, PHOTO_WIDTH, PHOTO_HEIGHT).catch((e) => {
            console.warn("face detection failed:", e);
            return null;
          })
        : Promise.resolve(null),
      wantsBackground
        ? extractSegmentationMask(rgba, PHOTO_WIDTH, PHOTO_HEIGHT).catch((e) => {
            console.warn("segmentation failed:", e);
            return null;
          })
        : Promise.resolve(null),
    ]);
    geometry = geomResult;
    segMask = maskResult;
  }

  // --- Skin mask (reused for blemish + beauty + whiten) ---
  let skinMask: Uint8Array | null = null;
  if (geometry && (beauty.skin || beauty.blemish || beauty.whiten)) {
    skinMask = wasm.build_skin_mask(
      new Uint8Array(rgba.buffer),
      PHOTO_WIDTH,
      PHOTO_HEIGHT,
      geometry.faceOval,
      geometry.exclusions,
    );
  }

  // --- Pipeline: blemish → beauty → eyes → slim → makeup → background → overlay ---

  if (beauty.blemish && skinMask) {
    rgba = new Uint8ClampedArray(
      wasm.remove_blemish(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        skinMask,
        clamp01(beauty.blemishStrength),
      ).buffer,
    );
  }

  if (beauty.skin && skinMask) {
    rgba = new Uint8ClampedArray(
      wasm.apply_beauty(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        skinMask,
        clamp01(beauty.strength),
      ).buffer,
    );
  }

  if (beauty.whiten && skinMask) {
    rgba = new Uint8ClampedArray(
      wasm.whiten_skin(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        skinMask,
        clamp01(beauty.whitenStrength),
      ).buffer,
    );
  }

  if (beauty.eyes && geometry) {
    const eyesGeom = computeEyesGeom(geometry.eyes, beauty.eyesStrength);
    const strength = Math.min(Math.max(0, beauty.eyesStrength), 1);
    rgba = new Uint8ClampedArray(
      wasm.enlarge_eyes(new Uint8Array(rgba.buffer), PHOTO_WIDTH, PHOTO_HEIGHT, eyesGeom, strength)
        .buffer,
    );
  }

  if (beauty.eyeSparkle && geometry) {
    rgba = new Uint8ClampedArray(
      wasm.apply_eye_sparkle(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        geometry.eyes,
        geometry.leftEye,
        geometry.rightEye,
        clamp01(beauty.eyeSparkleStrength),
      ).buffer,
    );
  }

  if (beauty.slim && geometry && beauty.slimStrength > 0) {
    rgba = new Uint8ClampedArray(
      wasm.slim_face(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        geometry.faceOval,
        clamp01(beauty.slimStrength),
      ).buffer,
    );
  }

  if (geometry && (makeup.lipEnabled || makeup.eyeShadowEnabled || makeup.blushEnabled)) {
    const makeupJson = JSON.stringify(buildMakeupJson(makeup));
    rgba = new Uint8ClampedArray(
      wasm.apply_makeup(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        geometry.lipsOuter,
        geometry.leftEye,
        geometry.rightEye,
        geometry.cheeks,
        makeupJson,
      ).buffer,
    );
  }

  if (wantsBackground) {
    const mask = segMask ?? new Float32Array(PHOTO_WIDTH * PHOTO_HEIGHT).fill(1);
    const [r, g, b] = hexToRgb(background.solidColor);
    const replPixels =
      background.mode === "image" && background.imageDataUrl
        ? new Uint8Array((await loadRgba(background.imageDataUrl)).buffer)
        : new Uint8Array(0);
    rgba = new Uint8ClampedArray(
      (
        await wasm.process_background(
          new Uint8Array(rgba.buffer),
          PHOTO_WIDTH,
          PHOTO_HEIGHT,
          mask,
          background.mode,
          r,
          g,
          b,
          replPixels,
          background.blurRadius,
        )
      ).buffer,
    );
  }

  if (tone.overlayEnabled) {
    const [r, g, b] = hexToRgb(tone.overlayColor);
    rgba = new Uint8ClampedArray(
      wasm.apply_color_overlay(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        r,
        g,
        b,
        clamp01(tone.overlayStrength),
        tone.overlayBlendMode,
        clamp01(tone.vignetteStrength),
      ).buffer,
    );
  } else if (tone.vignetteStrength > 0) {
    rgba = new Uint8ClampedArray(
      wasm.apply_color_overlay(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        0,
        0,
        0,
        0,
        "normal",
        clamp01(tone.vignetteStrength),
      ).buffer,
    );
  }

  if (tone.lutPreset !== "none") {
    rgba = new Uint8ClampedArray(
      (
        await wasm.apply_lut3d(
          new Uint8Array(rgba.buffer),
          PHOTO_WIDTH,
          PHOTO_HEIGHT,
          tone.lutPreset,
        )
      ).buffer,
    );
  }

  if (filter !== "none") {
    rgba = new Uint8ClampedArray(
      wasm.apply_filter(new Uint8Array(rgba.buffer), PHOTO_WIDTH, PHOTO_HEIGHT, filter).buffer,
    );
  }

  return toDataUrl(rgba);
}

export async function renderFinal(processedDataUrl: string, items: DekoItem[]): Promise<string> {
  const c = document.createElement("canvas");
  c.width = PHOTO_WIDTH;
  c.height = PHOTO_HEIGHT;
  const ctx = c.getContext("2d")!;

  await new Promise<void>((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      ctx.drawImage(img, 0, 0);
      resolve();
    };
    img.onerror = reject;
    img.src = processedDataUrl;
  });

  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.font = "48px serif";
  for (const item of items) {
    ctx.fillText(item.emoji, item.x * PHOTO_WIDTH, item.y * PHOTO_HEIGHT);
  }

  return c.toDataURL("image/png");
}
