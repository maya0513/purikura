import { beautyParams } from "~/state/signals";
import { setBeautyParam } from "~/hooks/useAppState";
import type { BeautyParams } from "~/state/types";

type StrengthKey = "strength" | "blemishStrength" | "eyesStrength";
type ToggleKey = "skin" | "blemish" | "eyes";

const OPTIONS: { toggleKey: ToggleKey; strengthKey: StrengthKey; label: string; emoji: string }[] = [
  { toggleKey: "skin", strengthKey: "strength", label: "美肌", emoji: "🌸" },
  { toggleKey: "blemish", strengthKey: "blemishStrength", label: "シミ取り", emoji: "✨" },
  { toggleKey: "eyes", strengthKey: "eyesStrength", label: "目拡大", emoji: "👁️" },
];

export function BeautyPanel() {
  const beauty = beautyParams.value;

  return (
    <div class="flex flex-col gap-1 h-full justify-center">
      {OPTIONS.map(({ toggleKey, strengthKey, label, emoji }) => {
        const enabled = beauty[toggleKey];
        const strength = beauty[strengthKey];
        const percent = Math.round(strength * 100);
        return (
          <div key={toggleKey} class="flex items-center gap-2">
            <button
              class={`flex items-center gap-1 px-2 py-1 rounded-lg font-medium text-xs transition-all min-w-20 justify-center ${
                enabled
                  ? "bg-bubblegum text-white shadow-sm"
                  : "bg-gray-100 text-gray-500 hover:bg-gray-200"
              }`}
              onClick={() => setBeautyParam(toggleKey, !enabled)}
            >
              <span>{emoji}</span>
              <span>{label}</span>
            </button>
            <input
              type="range"
              min={0}
              max={100}
              step={5}
              value={percent}
              disabled={!enabled}
              class="flex-1 accent-bubblegum disabled:opacity-40"
              onInput={(e) =>
                setBeautyParam(
                  strengthKey as keyof BeautyParams,
                  (e.currentTarget as HTMLInputElement).valueAsNumber / 100,
                )
              }
            />
            <span
              class={`w-9 text-right tabular-nums text-xs ${enabled ? "text-gray-600" : "text-gray-300"}`}
            >
              {percent}%
            </span>
          </div>
        );
      })}
    </div>
  );
}
