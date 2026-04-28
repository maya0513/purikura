import { useWasm } from "~/hooks/useWasm";
import { appState } from "~/state/signals";
import { CameraView } from "~/components/Camera";
import { EditView } from "~/components/PhotoStrip";

export function App() {
  const { error: wasmError } = useWasm();
  const state = appState.value;

  if (wasmError) {
    return (
      <div class="h-dvh flex items-center justify-center">
        <p class="text-red-500 px-4 text-center">WASM読み込みエラー: {wasmError}</p>
      </div>
    );
  }

  return (
    <div class="h-dvh flex flex-col overflow-hidden bg-cream">
      <header class="h-10 shrink-0 flex items-center justify-center border-b border-candy-pink/40">
        <h1 class="text-xl font-bold text-bubblegum" style={{ fontFamily: "var(--font-display)" }}>
          📸 プリクラ
        </h1>
      </header>
      <div class="flex-1 min-h-0">
        {(state === "idle" || state === "countdown" || state === "capturing") && <CameraView />}
        {state === "edit" && <EditView />}
      </div>
    </div>
  );
}
