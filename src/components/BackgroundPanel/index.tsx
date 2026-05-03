import { backgroundParams } from "~/state/signals";
import { setBackgroundParam } from "~/hooks/useAppState";
import type { BackgroundMode } from "~/state/types";

const MODES: { id: BackgroundMode; label: string; emoji: string }[] = [
  { id: "none", label: "そのまま", emoji: "🖼️" },
  { id: "blur", label: "ぼかし", emoji: "🌫️" },
  { id: "solid", label: "単色", emoji: "🎨" },
  { id: "image", label: "画像", emoji: "🏞️" },
];

export function BackgroundPanel() {
  const bg = backgroundParams.value;

  function handleImageUpload(e: Event) {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      setBackgroundParam("imageDataUrl", reader.result as string);
    };
    reader.readAsDataURL(file);
  }

  return (
    <div class="flex flex-col gap-2 h-full overflow-y-auto">
      {/* Mode selector */}
      <div class="grid grid-cols-4 gap-1">
        {MODES.map((m) => (
          <button
            key={m.id}
            class={`flex flex-col items-center py-1 rounded-lg text-xs font-medium transition-all gap-0.5 ${
              bg.mode === m.id
                ? "bg-bubblegum text-white shadow-sm"
                : "bg-gray-100 text-gray-600 hover:bg-gray-200"
            }`}
            onClick={() => setBackgroundParam("mode", m.id)}
          >
            <span class="text-base leading-none">{m.emoji}</span>
            <span>{m.label}</span>
          </button>
        ))}
      </div>

      {/* Mode-specific settings */}
      {bg.mode === "blur" && (
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500 min-w-14">ぼかし強度</span>
          <input
            type="range"
            min={2}
            max={40}
            step={2}
            value={bg.blurRadius}
            class="flex-1 accent-bubblegum"
            onInput={(e) =>
              setBackgroundParam("blurRadius", (e.currentTarget as HTMLInputElement).valueAsNumber)
            }
          />
          <span class="w-8 text-right tabular-nums text-xs text-gray-600">{bg.blurRadius}px</span>
        </div>
      )}

      {bg.mode === "solid" && (
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500 min-w-14">背景色</span>
          <input
            type="color"
            value={bg.solidColor}
            class="w-12 h-8 rounded cursor-pointer border border-gray-200"
            onInput={(e) =>
              setBackgroundParam("solidColor", (e.currentTarget as HTMLInputElement).value)
            }
          />
          <span class="text-xs text-gray-400">{bg.solidColor}</span>
        </div>
      )}

      {bg.mode === "image" && (
        <div class="flex flex-col gap-1">
          <label class="flex items-center gap-2 cursor-pointer">
            <span class="btn-primary py-1 px-3 text-xs">📁 画像を選択</span>
            <input type="file" accept="image/*" class="hidden" onInput={handleImageUpload} />
          </label>
          {bg.imageDataUrl && (
            <div class="flex items-center gap-2">
              <img
                src={bg.imageDataUrl}
                alt="背景"
                class="w-12 h-8 object-cover rounded border border-gray-200"
              />
              <span class="text-xs text-gray-500">背景画像設定済み</span>
              <button
                class="ml-auto text-xs text-gray-400 hover:text-gray-600"
                onClick={() => setBackgroundParam("imageDataUrl", null)}
              >
                ✕ 削除
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
