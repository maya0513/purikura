export type AppState = "idle" | "countdown" | "capturing" | "edit";

export type FilterName = "none" | "grayscale" | "sepia" | "vivid" | "soft" | "warm" | "cool";

export type LutPreset = "none" | "natural" | "pop" | "soft" | "film" | "vintage" | "cool" | "peach";

export type BackgroundMode = "none" | "blur" | "solid" | "image";

export type BlendMode = "normal" | "multiply" | "screen" | "softlight";

export interface BeautyParams {
  skin: boolean;
  blemish: boolean;
  eyes: boolean;
  slim: boolean;
  whiten: boolean;
  eyeSparkle: boolean;
  strength: number;
  blemishStrength: number;
  eyesStrength: number;
  slimStrength: number;
  whitenStrength: number;
  eyeSparkleStrength: number;
}

export interface MakeupParams {
  lipEnabled: boolean;
  lipColor: string;
  lipStrength: number;
  eyeShadowEnabled: boolean;
  eyeShadowColor: string;
  eyeShadowStrength: number;
  blushEnabled: boolean;
  blushColor: string;
  blushStrength: number;
}

export interface ToneParams {
  lutPreset: LutPreset;
  overlayEnabled: boolean;
  overlayColor: string;
  overlayStrength: number;
  overlayBlendMode: BlendMode;
  vignetteStrength: number;
}

export interface BackgroundParams {
  mode: BackgroundMode;
  blurRadius: number;
  solidColor: string;
  imageDataUrl: string | null;
}

export interface DekoItem {
  id: string;
  emoji: string;
  x: number;
  y: number;
}

export const PHOTO_WIDTH = 640;
export const PHOTO_HEIGHT = 480;

// Decode a CSS hex colour "#rrggbb" → [r, g, b] as 0..255 integers.
export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16) || 0;
  const g = parseInt(h.slice(2, 4), 16) || 0;
  const b = parseInt(h.slice(4, 6), 16) || 0;
  return [r, g, b];
}
