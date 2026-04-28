import { useEffect, useState } from "preact/hooks";
import { initWasm } from "~/lib/imageProcessor";
import { markWasmReady } from "~/hooks/useAppState";

export function useWasm(): { isReady: boolean; error: string | null } {
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    initWasm()
      .then(() => {
        markWasmReady();
        setIsReady(true);
      })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : "WASM初期化に失敗しました");
      });
  }, []);

  return { isReady, error };
}
