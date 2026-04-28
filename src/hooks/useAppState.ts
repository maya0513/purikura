import {
  appState,
  capturedPhoto,
  selectedFilter,
  selectedFrame,
  beautyParams,
  dekoItems,
  countdownValue,
  processedUrl,
  wasmReady,
} from "~/state/signals";
import {
  nextStateOnStart,
  nextStateOnCapture,
  nextStateOnFinish,
  nextStateOnReset,
} from "~/lib/stateMachine";
import type { FilterName, FrameName, BeautyParams, DekoItem } from "~/state/types";

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
  selectedFrame.value = "none";
  beautyParams.value = {
    skin: true,
    blemish: true,
    eyes: true,
    strength: 0.85,
    blemishStrength: 0.95,
    eyesStrength: 0.55,
  };
  dekoItems.value = [];
  processedUrl.value = null;
}

export function setFilter(filter: FilterName): void {
  selectedFilter.value = filter;
}

export function setFrame(frame: FrameName): void {
  selectedFrame.value = frame;
}

export function setBeautyParam<K extends keyof BeautyParams>(key: K, value: BeautyParams[K]): void {
  beautyParams.value = { ...beautyParams.value, [key]: value };
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

export function tickCountdown(value: number): void {
  countdownValue.value = value;
}
