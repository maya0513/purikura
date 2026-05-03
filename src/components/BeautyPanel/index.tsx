import { beautyParams } from "~/state/signals";
import { setBeautyParam } from "~/hooks/useAppState";
import type { BeautyParams } from "~/state/types";

type StrengthKey =
  | "strength"
  | "blemishStrength"
  | "eyesStrength"
  | "slimStrength"
  | "whitenStrength"
  | "eyeSparkleStrength";
type ToggleKey = "skin" | "blemish" | "eyes" | "slim" | "whiten" | "eyeSparkle";

interface Option {
  toggleKey: ToggleKey;
  strengthKey: StrengthKey;
  label: string;
  emoji: string;
  maxPercent: number;
}

const OPTIONS: Option[] = [
  { toggleKey: "skin", strengthKey: "strength", label: "美肌", emoji: "🌸", maxPercent: 100 },
  {
    toggleKey: "blemish",
    strengthKey: "blemishStrength",
    label: "シミ取り",
    emoji: "🔮",
    maxPercent: 100,
  },
  {
    toggleKey: "whiten",
    strengthKey: "whitenStrength",
    label: "白肌",
    emoji: "🤍",
    maxPercent: 100,
  },
  { toggleKey: "eyes", strengthKey: "eyesStrength", label: "目拡大", emoji: "👁️", maxPercent: 200 },
  {
    toggleKey: "eyeSparkle",
    strengthKey: "eyeSparkleStrength",
    label: "目キラキラ",
    emoji: "✨",
    maxPercent: 100,
  },
  { toggleKey: "slim", strengthKey: "slimStrength", label: "小顔", emoji: "🫶", maxPercent: 100 },
];

export function BeautyPanel() {
  const beauty = beautyParams.value;

  return (
    <div class="flex flex-col gap-1 h-full justify-center">
      {OPTIONS.map(({ toggleKey, strengthKey, label, emoji, maxPercent }) => {
        const enabled = beauty[toggleKey];
        const strength = beauty[strengthKey];
        const percent = Math.round(strength * 100);
        const extreme = percent > 100;
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
              max={maxPercent}
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
              class={`w-10 text-right tabular-nums text-xs ${
                !enabled ? "text-gray-300" : extreme ? "text-bubblegum font-bold" : "text-gray-600"
              }`}
            >
              {percent}%
            </span>
          </div>
        );
      })}
    </div>
  );
}
