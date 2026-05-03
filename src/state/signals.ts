import { signal, computed } from "@preact/signals";
import type {
  AppState,
  FilterName,
  BeautyParams,
  MakeupParams,
  ToneParams,
  BackgroundParams,
  DekoItem,
} from "./types";

export const appState = signal<AppState>("idle");
export const capturedPhoto = signal<string | null>(null);
export const selectedFilter = signal<FilterName>("none");

export const beautyParams = signal<BeautyParams>({
  skin: true,
  blemish: true,
  eyes: false,
  slim: false,
  whiten: false,
  eyeSparkle: false,
  strength: 0.85,
  blemishStrength: 0.95,
  eyesStrength: 0.55,
  slimStrength: 0.35,
  whitenStrength: 0.75,
  eyeSparkleStrength: 0.65,
});

export const makeupParams = signal<MakeupParams>({
  lipEnabled: false,
  lipColor: "#e84c6e",
  lipStrength: 0.5,
  eyeShadowEnabled: false,
  eyeShadowColor: "#9b72cf",
  eyeShadowStrength: 0.35,
  blushEnabled: false,
  blushColor: "#ff9090",
  blushStrength: 0.3,
});

export const toneParams = signal<ToneParams>({
  lutPreset: "none",
  overlayEnabled: false,
  overlayColor: "#ff88cc",
  overlayStrength: 0.15,
  overlayBlendMode: "softlight",
  vignetteStrength: 0.0,
});

export const backgroundParams = signal<BackgroundParams>({
  mode: "none",
  blurRadius: 15,
  solidColor: "#ffffff",
  imageDataUrl: null,
});

export const dekoItems = signal<DekoItem[]>([]);
export const countdownValue = signal<number>(3);
export const wasmReady = signal<boolean>(false);
export const gpuReady = signal<boolean>(false);
export const processedUrl = signal<string | null>(null);

export const canStartCapture = computed(() => appState.value === "idle" && wasmReady.value);
