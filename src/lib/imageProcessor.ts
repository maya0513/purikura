import type { FilterName, FrameName, BeautyParams, DekoItem } from "~/state/types";
import { PHOTO_WIDTH, PHOTO_HEIGHT } from "~/state/types";
import { extractGeometry, type FaceGeometry } from "~/lib/faceLandmarks";

type WasmModule = {
  apply_filter: (pixels: Uint8Array, w: number, h: number, filter: string) => Uint8Array;
  compose_frame: (photo: Uint8Array, frame: Uint8Array, w: number, h: number) => Uint8Array;
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
};

let wasm: WasmModule | null = null;

export async function initWasm(): Promise<void> {
  if (wasm) return;
  const mod = await import("~/wasm/pkg/purikura_wasm");
  await mod.default();
  wasm = mod as unknown as WasmModule;
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

const BLEMISH_STRENGTH = 0.95;
const EYE_STRENGTH = 0.55;

export async function processPhoto(
  dataUrl: string,
  filter: FilterName,
  frame: FrameName,
  beauty: BeautyParams,
): Promise<string> {
  if (!wasm) throw new Error("WASM not initialized");

  let rgba = await loadRgba(dataUrl);

  const wantsFaceWork = beauty.skin || beauty.blemish || beauty.eyes;
  let geometry: FaceGeometry | null = null;
  if (wantsFaceWork) {
    try {
      geometry = await extractGeometry(rgba, PHOTO_WIDTH, PHOTO_HEIGHT);
    } catch (e) {
      // Detector load failure or runtime error — skip beauty silently rather
      // than wedge the whole pipeline. The user still gets a photo.
      console.warn("face landmark detection failed:", e);
      geometry = null;
    }
  }

  // Build the skin mask once and reuse for both blemish + skin smoothing.
  let skinMask: Uint8Array | null = null;
  if (geometry && (beauty.skin || beauty.blemish)) {
    skinMask = wasm.build_skin_mask(
      new Uint8Array(rgba.buffer),
      PHOTO_WIDTH,
      PHOTO_HEIGHT,
      geometry.faceOval,
      geometry.exclusions,
    );
  }

  // Pipeline order: blemish → skin smoothing → eyes (per image-processing
  // research; see /tmp/purikura-eval diagnostics).
  if (beauty.blemish && skinMask) {
    rgba = new Uint8ClampedArray(
      wasm.remove_blemish(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        skinMask,
        BLEMISH_STRENGTH,
      ).buffer,
    );
  }

  if (beauty.skin && skinMask) {
    const strength = clamp01(beauty.strength ?? 0.85);
    rgba = new Uint8ClampedArray(
      wasm.apply_beauty(new Uint8Array(rgba.buffer), PHOTO_WIDTH, PHOTO_HEIGHT, skinMask, strength)
        .buffer,
    );
  }

  if (beauty.eyes && geometry) {
    rgba = new Uint8ClampedArray(
      wasm.enlarge_eyes(
        new Uint8Array(rgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
        geometry.eyes,
        EYE_STRENGTH,
      ).buffer,
    );
  }

  if (filter !== "none") {
    rgba = new Uint8ClampedArray(
      wasm.apply_filter(new Uint8Array(rgba.buffer), PHOTO_WIDTH, PHOTO_HEIGHT, filter).buffer,
    );
  }

  if (frame !== "none") {
    const frameRgba = await loadRgba(`/frames/${frame}.png`);
    rgba = new Uint8ClampedArray(
      wasm.compose_frame(
        new Uint8Array(rgba.buffer),
        new Uint8Array(frameRgba.buffer),
        PHOTO_WIDTH,
        PHOTO_HEIGHT,
      ).buffer,
    );
  }

  return toDataUrl(rgba);
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
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
