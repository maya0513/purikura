import type { AppState } from "~/state/types";

export const nextStateOnStart = (s: AppState): AppState => (s === "idle" ? "countdown" : s);

export const nextStateOnCapture = (s: AppState): AppState => (s === "countdown" ? "capturing" : s);

export const nextStateOnFinish = (s: AppState): AppState => (s === "capturing" ? "edit" : s);

export const nextStateOnReset = (_s: AppState): AppState => "idle";

export const canStartCapture = (s: AppState, ready: boolean): boolean => s === "idle" && ready;

export const canFinishCapture = (n: number): boolean => n >= 1;
