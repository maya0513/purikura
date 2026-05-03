// Browser-side entry point for Playwright-driven sample generation.
// Vite processes this file so the ~ alias and WASM plugins work normally.
// Playwright opens /scripts/browser_processor.html, waits for window.__ready,
// then calls window.__processPhoto(dataUrl, filter, beauty, makeup, tone, bg).

import { initWasm, initGpu, processPhoto } from "~/lib/imageProcessor";
import type {
  FilterName,
  BeautyParams,
  MakeupParams,
  ToneParams,
  BackgroundParams,
} from "~/state/types";

declare global {
  interface Window {
    __processPhoto: (
      dataUrl: string,
      filter: FilterName,
      beauty: BeautyParams,
      makeup: MakeupParams,
      tone: ToneParams,
      bg: BackgroundParams,
    ) => Promise<string>;
    __ready: boolean;
    __initError: string | null;
  }
}

window.__ready = false;
window.__initError = null;

(async () => {
  try {
    await initWasm();
    if (typeof navigator !== "undefined" && navigator.gpu != null) {
      try {
        await initGpu();
      } catch {
        // GPU unavailable — CPU fallback is fine
      }
    }
    window.__processPhoto = processPhoto;
    window.__ready = true;
  } catch (e) {
    window.__initError = String(e);
    console.error("processor init failed:", e);
  }
})();
