import { makeupParams } from "~/state/signals";
import { setMakeupParam } from "~/hooks/useAppState";

export function MakeupPanel() {
  const mk = makeupParams.value;

  return (
    <div class="flex flex-col gap-1.5 h-full justify-center overflow-y-auto">
      {/* Lip */}
      <div class="flex items-center gap-2">
        <button
          class={`flex items-center gap-1 px-2 py-1 rounded-lg font-medium text-xs transition-all min-w-20 justify-center ${mk.lipEnabled ? "bg-bubblegum text-white shadow-sm" : "bg-gray-100 text-gray-500 hover:bg-gray-200"}`}
          onClick={() => setMakeupParam("lipEnabled", !mk.lipEnabled)}
        >
          <span>💋</span>
          <span>リップ</span>
        </button>
        <input
          type="color"
          value={mk.lipColor}
          disabled={!mk.lipEnabled}
          class="w-8 h-7 rounded cursor-pointer disabled:opacity-40 border border-gray-200"
          onInput={(e) => setMakeupParam("lipColor", (e.currentTarget as HTMLInputElement).value)}
        />
        <input
          type="range"
          min={0}
          max={100}
          step={5}
          value={Math.round(mk.lipStrength * 100)}
          disabled={!mk.lipEnabled}
          class="flex-1 accent-bubblegum disabled:opacity-40"
          onInput={(e) =>
            setMakeupParam("lipStrength", (e.currentTarget as HTMLInputElement).valueAsNumber / 100)
          }
        />
        <span
          class={`w-10 text-right tabular-nums text-xs ${mk.lipEnabled ? "text-gray-600" : "text-gray-300"}`}
        >
          {Math.round(mk.lipStrength * 100)}%
        </span>
      </div>

      {/* Eye shadow */}
      <div class="flex items-center gap-2">
        <button
          class={`flex items-center gap-1 px-2 py-1 rounded-lg font-medium text-xs transition-all min-w-20 justify-center ${mk.eyeShadowEnabled ? "bg-bubblegum text-white shadow-sm" : "bg-gray-100 text-gray-500 hover:bg-gray-200"}`}
          onClick={() => setMakeupParam("eyeShadowEnabled", !mk.eyeShadowEnabled)}
        >
          <span>👁️</span>
          <span>アイシャドウ</span>
        </button>
        <input
          type="color"
          value={mk.eyeShadowColor}
          disabled={!mk.eyeShadowEnabled}
          class="w-8 h-7 rounded cursor-pointer disabled:opacity-40 border border-gray-200"
          onInput={(e) =>
            setMakeupParam("eyeShadowColor", (e.currentTarget as HTMLInputElement).value)
          }
        />
        <input
          type="range"
          min={0}
          max={100}
          step={5}
          value={Math.round(mk.eyeShadowStrength * 100)}
          disabled={!mk.eyeShadowEnabled}
          class="flex-1 accent-bubblegum disabled:opacity-40"
          onInput={(e) =>
            setMakeupParam(
              "eyeShadowStrength",
              (e.currentTarget as HTMLInputElement).valueAsNumber / 100,
            )
          }
        />
        <span
          class={`w-10 text-right tabular-nums text-xs ${mk.eyeShadowEnabled ? "text-gray-600" : "text-gray-300"}`}
        >
          {Math.round(mk.eyeShadowStrength * 100)}%
        </span>
      </div>

      {/* Blush */}
      <div class="flex items-center gap-2">
        <button
          class={`flex items-center gap-1 px-2 py-1 rounded-lg font-medium text-xs transition-all min-w-20 justify-center ${mk.blushEnabled ? "bg-bubblegum text-white shadow-sm" : "bg-gray-100 text-gray-500 hover:bg-gray-200"}`}
          onClick={() => setMakeupParam("blushEnabled", !mk.blushEnabled)}
        >
          <span>🌸</span>
          <span>チーク</span>
        </button>
        <input
          type="color"
          value={mk.blushColor}
          disabled={!mk.blushEnabled}
          class="w-8 h-7 rounded cursor-pointer disabled:opacity-40 border border-gray-200"
          onInput={(e) => setMakeupParam("blushColor", (e.currentTarget as HTMLInputElement).value)}
        />
        <input
          type="range"
          min={0}
          max={100}
          step={5}
          value={Math.round(mk.blushStrength * 100)}
          disabled={!mk.blushEnabled}
          class="flex-1 accent-bubblegum disabled:opacity-40"
          onInput={(e) =>
            setMakeupParam(
              "blushStrength",
              (e.currentTarget as HTMLInputElement).valueAsNumber / 100,
            )
          }
        />
        <span
          class={`w-10 text-right tabular-nums text-xs ${mk.blushEnabled ? "text-gray-600" : "text-gray-300"}`}
        >
          {Math.round(mk.blushStrength * 100)}%
        </span>
      </div>
    </div>
  );
}
