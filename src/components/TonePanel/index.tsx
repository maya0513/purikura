import { toneParams } from "~/state/signals";
import { setToneParam } from "~/hooks/useAppState";
import type { LutPreset, BlendMode } from "~/state/types";

const LUT_PRESETS: { id: LutPreset; label: string }[] = [
  { id: "none", label: "なし" },
  { id: "natural", label: "ナチュラル" },
  { id: "pop", label: "ポップ" },
  { id: "soft", label: "ソフト" },
  { id: "film", label: "フィルム" },
  { id: "vintage", label: "ヴィンテージ" },
  { id: "cool", label: "クール" },
  { id: "peach", label: "ピーチ" },
];

const BLEND_MODES: { id: BlendMode; label: string }[] = [
  { id: "normal", label: "通常" },
  { id: "multiply", label: "乗算" },
  { id: "screen", label: "スクリーン" },
  { id: "softlight", label: "ソフトライト" },
];

export function TonePanel() {
  const tone = toneParams.value;

  return (
    <div class="flex flex-col gap-2 h-full overflow-y-auto">
      {/* LUT preset grid */}
      <div class="grid grid-cols-4 gap-1">
        {LUT_PRESETS.map((p) => (
          <button
            key={p.id}
            class={`px-1 py-1 rounded-lg text-xs font-medium transition-all ${
              tone.lutPreset === p.id
                ? "bg-bubblegum text-white shadow-sm"
                : "bg-gray-100 text-gray-600 hover:bg-gray-200"
            }`}
            onClick={() => setToneParam("lutPreset", p.id)}
          >
            {p.label}
          </button>
        ))}
      </div>

      <div class="border-t border-candy-pink/20 pt-1.5 flex flex-col gap-1.5">
        {/* Color overlay */}
        <div class="flex items-center gap-2">
          <button
            class={`flex items-center gap-1 px-2 py-1 rounded-lg font-medium text-xs transition-all min-w-[5.5rem] justify-center ${tone.overlayEnabled ? "bg-bubblegum text-white shadow-sm" : "bg-gray-100 text-gray-500 hover:bg-gray-200"}`}
            onClick={() => setToneParam("overlayEnabled", !tone.overlayEnabled)}
          >
            <span>🎨</span>
            <span>オーバーレイ</span>
          </button>
          <input
            type="color"
            value={tone.overlayColor}
            disabled={!tone.overlayEnabled}
            class="w-8 h-7 rounded cursor-pointer disabled:opacity-40 border border-gray-200"
            onInput={(e) =>
              setToneParam("overlayColor", (e.currentTarget as HTMLInputElement).value)
            }
          />
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={Math.round(tone.overlayStrength * 100)}
            disabled={!tone.overlayEnabled}
            class="flex-1 accent-bubblegum disabled:opacity-40"
            onInput={(e) =>
              setToneParam(
                "overlayStrength",
                (e.currentTarget as HTMLInputElement).valueAsNumber / 100,
              )
            }
          />
          <span
            class={`w-10 text-right tabular-nums text-xs ${tone.overlayEnabled ? "text-gray-600" : "text-gray-300"}`}
          >
            {Math.round(tone.overlayStrength * 100)}%
          </span>
        </div>

        {/* Blend mode selector */}
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500 min-w-[5.5rem]">ブレンド</span>
          <select
            disabled={!tone.overlayEnabled}
            value={tone.overlayBlendMode}
            class="flex-1 text-xs border border-gray-200 rounded-lg px-2 py-1 bg-white disabled:opacity-40"
            onChange={(e) =>
              setToneParam(
                "overlayBlendMode",
                (e.currentTarget as HTMLSelectElement).value as BlendMode,
              )
            }
          >
            {BLEND_MODES.map((m) => (
              <option key={m.id} value={m.id}>
                {m.label}
              </option>
            ))}
          </select>
        </div>

        {/* Vignette */}
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500 min-w-[5.5rem]">ビネット</span>
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={Math.round(tone.vignetteStrength * 100)}
            class="flex-1 accent-bubblegum"
            onInput={(e) =>
              setToneParam(
                "vignetteStrength",
                (e.currentTarget as HTMLInputElement).valueAsNumber / 100,
              )
            }
          />
          <span class="w-10 text-right tabular-nums text-xs text-gray-600">
            {Math.round(tone.vignetteStrength * 100)}%
          </span>
        </div>
      </div>
    </div>
  );
}
