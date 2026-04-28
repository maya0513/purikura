import { countdownValue, appState } from "~/state/signals";

export function CountdownOverlay() {
  const state = appState.value;
  if (state !== "countdown" && state !== "capturing") return null;

  return (
    <div
      data-testid="countdown-overlay"
      class="absolute inset-0 flex items-center justify-center pointer-events-none"
    >
      {state === "capturing" ? (
        <div class="absolute inset-0 bg-white opacity-70 animate-flash" />
      ) : (
        <span class="text-9xl font-bold text-white drop-shadow-lg select-none">
          {countdownValue.value === 0 ? "📸" : countdownValue.value}
        </span>
      )}
    </div>
  );
}
