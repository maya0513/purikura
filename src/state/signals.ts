import { signal, computed } from "@preact/signals";
import type { AppState, FilterName, FrameName, BeautyParams, DekoItem } from "./types";

export const appState = signal<AppState>("idle");
export const capturedPhoto = signal<string | null>(null);
export const selectedFilter = signal<FilterName>("none");
export const selectedFrame = signal<FrameName>("none");
export const beautyParams = signal<BeautyParams>({
  skin: true,
  blemish: true,
  eyes: false,
  strength: 0.85,
  blemishStrength: 0.95,
  eyesStrength: 0.55,
});
export const dekoItems = signal<DekoItem[]>([]);
export const countdownValue = signal<number>(3);
export const wasmReady = signal<boolean>(false);
export const processedUrl = signal<string | null>(null);

export const canStartCapture = computed(() => appState.value === "idle" && wasmReady.value);
