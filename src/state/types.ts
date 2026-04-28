export type AppState = "idle" | "countdown" | "capturing" | "edit";

export type FilterName = "none" | "grayscale" | "sepia" | "vivid" | "soft" | "warm" | "cool";

export type FrameName = "none" | "hearts" | "stars" | "flowers" | "bubbles";

export interface BeautyParams {
  skin: boolean;
  blemish: boolean;
  eyes: boolean;
  /** Skin smoothing intensity 0..1. Defaults to 0.85 for プリクラ強め. */
  strength: number;
  /** Blemish removal intensity 0..1. */
  blemishStrength: number;
  /** Eye enlargement intensity 0..1. */
  eyesStrength: number;
}

export interface DekoItem {
  id: string;
  emoji: string;
  x: number;
  y: number;
}

export const PHOTO_WIDTH = 640;
export const PHOTO_HEIGHT = 480;
