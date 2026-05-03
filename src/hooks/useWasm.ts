import { useEffect, useState } from "preact/hooks";
import { initWasm, initGpu } from "~/lib/imageProcessor";
import { markWasmReady, markGpuReady } from "~/hooks/useAppState";

export function useWasm(): { isReady: boolean; error: string | null } {
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    initWasm()
      .then(async () => {
        markWasmReady();
        setIsReady(true);
        // GPU init is best-effort — failure falls back to CPU paths silently.
        await initGpu();
        markGpuReady();
      })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : "WASM初期化に失敗しました");
      });
  }, []);

  return { isReady, error };
}
