import {
  appState,
  capturedPhoto,
  selectedFilter,
  beautyParams,
  makeupParams,
  toneParams,
  backgroundParams,
  dekoItems,
  countdownValue,
  processedUrl,
  wasmReady,
  gpuReady,
} from "~/state/signals";
import {
  nextStateOnStart,
  nextStateOnCapture,
  nextStateOnFinish,
  nextStateOnReset,
} from "~/lib/stateMachine";
import type {
  FilterName,
  BeautyParams,
  MakeupParams,
  ToneParams,
  BackgroundParams,
  DekoItem,
} from "~/state/types";

export function startCountdown(): void {
  appState.value = nextStateOnStart(appState.value);
  countdownValue.value = 3;
}

export function beginCapture(): void {
  appState.value = nextStateOnCapture(appState.value);
}

export function capturePhoto(dataUrl: string): void {
  capturedPhoto.value = dataUrl;
}

export function finishCapture(): void {
  appState.value = nextStateOnFinish(appState.value);
}

export function reset(): void {
  appState.value = nextStateOnReset(appState.value);
  capturedPhoto.value = null;
  selectedFilter.value = "none";
  beautyParams.value = {
    skin: true,
    blemish: true,
    eyes: true,
    slim: false,
    strength: 0.85,
    blemishStrength: 0.95,
    eyesStrength: 0.55,
    slimStrength: 0.35,
  };
  makeupParams.value = {
    lipEnabled: false,
    lipColor: "#e84c6e",
    lipStrength: 0.5,
    eyeShadowEnabled: false,
    eyeShadowColor: "#9b72cf",
    eyeShadowStrength: 0.35,
    blushEnabled: false,
    blushColor: "#ff9090",
    blushStrength: 0.3,
  };
  toneParams.value = {
    lutPreset: "none",
    overlayEnabled: false,
    overlayColor: "#ff88cc",
    overlayStrength: 0.15,
    overlayBlendMode: "softlight",
    vignetteStrength: 0.0,
  };
  backgroundParams.value = {
    mode: "none",
    blurRadius: 15,
    solidColor: "#ffffff",
    imageDataUrl: null,
  };
  dekoItems.value = [];
  processedUrl.value = null;
}

export function setFilter(filter: FilterName): void {
  selectedFilter.value = filter;
}

export function setBeautyParam<K extends keyof BeautyParams>(key: K, value: BeautyParams[K]): void {
  beautyParams.value = { ...beautyParams.value, [key]: value };
}

export function setMakeupParam<K extends keyof MakeupParams>(key: K, value: MakeupParams[K]): void {
  makeupParams.value = { ...makeupParams.value, [key]: value };
}

export function setToneParam<K extends keyof ToneParams>(key: K, value: ToneParams[K]): void {
  toneParams.value = { ...toneParams.value, [key]: value };
}

export function setBackgroundParam<K extends keyof BackgroundParams>(
  key: K,
  value: BackgroundParams[K],
): void {
  backgroundParams.value = { ...backgroundParams.value, [key]: value };
}

export function addDekoItem(item: DekoItem): void {
  dekoItems.value = [...dekoItems.value, item];
}

export function removeDekoItem(id: string): void {
  dekoItems.value = dekoItems.value.filter((d) => d.id !== id);
}

export function setProcessedUrl(url: string): void {
  processedUrl.value = url;
}

export function markWasmReady(): void {
  wasmReady.value = true;
}

export function markGpuReady(): void {
  gpuReady.value = true;
}

export function tickCountdown(value: number): void {
  countdownValue.value = value;
}
