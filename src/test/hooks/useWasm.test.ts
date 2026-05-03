import { describe, expect, it, vi, beforeEach } from "vite-plus/test";
import { renderHook, waitFor } from "@testing-library/preact";
import { wasmReady, gpuReady } from "~/state/signals";

vi.mock("~/lib/imageProcessor", () => ({
  initWasm: vi.fn().mockResolvedValue(undefined),
  initGpu: vi.fn().mockResolvedValue(undefined),
}));

import { useWasm } from "~/hooks/useWasm";
import * as imageProcessor from "~/lib/imageProcessor";

beforeEach(() => {
  vi.clearAllMocks();
  wasmReady.value = false;
  gpuReady.value = false;
  vi.mocked(imageProcessor.initWasm).mockResolvedValue(undefined);
  vi.mocked(imageProcessor.initGpu).mockResolvedValue(undefined);
});

describe("useWasm", () => {
  it("初期値は isReady=false, error=null", () => {
    const { result } = renderHook(() => useWasm());
    expect(result.current.isReady).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("initWasm 完了後に isReady=true になる", async () => {
    const { result } = renderHook(() => useWasm());
    await waitFor(() => expect(result.current.isReady).toBe(true));
    expect(result.current.error).toBeNull();
  });

  it("initWasm 完了後に wasmReady シグナルが true になる", async () => {
    renderHook(() => useWasm());
    await waitFor(() => expect(wasmReady.value).toBe(true));
  });

  it("initGpu 完了後に gpuReady シグナルが true になる", async () => {
    renderHook(() => useWasm());
    await waitFor(() => expect(gpuReady.value).toBe(true));
  });

  it("initWasm 失敗（Error インスタンス）で error にメッセージが設定される", async () => {
    vi.mocked(imageProcessor.initWasm).mockRejectedValueOnce(new Error("load failed"));
    const { result } = renderHook(() => useWasm());
    await waitFor(() => expect(result.current.error).toBe("load failed"));
    expect(result.current.isReady).toBe(false);
  });

  it("initWasm 失敗（非 Error）でデフォルトメッセージが設定される", async () => {
    vi.mocked(imageProcessor.initWasm).mockRejectedValueOnce("string error");
    const { result } = renderHook(() => useWasm());
    await waitFor(() => expect(result.current.error).toBe("WASM初期化に失敗しました"));
  });
});
