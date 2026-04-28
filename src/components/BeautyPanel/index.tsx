import { beautyParams } from "~/state/signals";
import { setBeautyParam } from "~/hooks/useAppState";
import type { BeautyParams } from "~/state/types";

const OPTIONS: { key: keyof BeautyParams; label: string; emoji: string }[] = [
  { key: "skin", label: "美肌", emoji: "🌸" },
  { key: "blemish", label: "シミ取り", emoji: "✨" },
  { key: "eyes", label: "目拡大", emoji: "👁️" },
];

export function BeautyPanel() {
  const beauty = beautyParams.value;

  return (
    <div class="flex flex-col gap-2 h-full justify-center">
      <div class="flex gap-2 items-center">
        {OPTIONS.map(({ key, label, emoji }) => (
          <button
            key={key}
            class={`flex items-center gap-1.5 px-3 py-2 rounded-xl font-medium text-sm transition-all ${
              beauty[key]
                ? "bg-bubblegum text-white shadow-sm scale-105"
                : "bg-gray-100 text-gray-500 hover:bg-gray-200"
            }`}
            onClick={() => setBeautyParam(key, !beauty[key])}
          >
            <span>{emoji}</span>
            <span>{label}</span>
          </button>
        ))}
      </div>
      {beauty.skin && (
        <label class="flex items-center gap-2 px-1 text-xs text-gray-600">
          <span class="whitespace-nowrap">強度</span>
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={Math.round(beauty.strength * 100)}
            class="flex-1 accent-bubblegum"
            onInput={(e) =>
              setBeautyParam("strength", (e.currentTarget as HTMLInputElement).valueAsNumber / 100)
            }
          />
          <span class="w-8 text-right tabular-nums">{Math.round(beauty.strength * 100)}%</span>
        </label>
      )}
    </div>
  );
}
