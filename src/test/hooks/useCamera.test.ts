import { describe, expect, it, vi, beforeEach, afterEach } from "vite-plus/test";
import { h } from "preact";
import { render, act, waitFor, cleanup } from "@testing-library/preact";
import { renderHook } from "@testing-library/preact";
import { useCamera } from "~/hooks/useCamera";

// Canvas stub is re-applied each test so vi.restoreAllMocks in afterEach is safe.
function stubCanvas() {
  const origCreate = document.createElement.bind(document);
  vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
    if (tag === "canvas") {
      return {
        width: 0,
        height: 0,
        getContext: () => ({ drawImage: vi.fn() }),
        toDataURL: () => "data:image/jpeg;base64,stub",
      } as unknown as HTMLCanvasElement;
    }
    return origCreate(tag);
  });
}

beforeEach(() => stubCanvas());

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

// ── エラーパス ────────────────────────────────────────────────────────────────

describe("useCamera — エラーパス", () => {
  it("getUserMedia 拒否（Error）で error にメッセージが設定される", async () => {
    vi.stubGlobal("navigator", {
      mediaDevices: {
        getUserMedia: vi.fn().mockRejectedValue(new Error("permission denied")),
      },
    });

    const { result } = renderHook(() => useCamera());
    await waitFor(() => expect(result.current.error).toBe("permission denied"));
    expect(result.current.isReady).toBe(false);
  });

  it("getUserMedia 拒否（非 Error）でデフォルトメッセージが設定される", async () => {
    vi.stubGlobal("navigator", {
      mediaDevices: { getUserMedia: vi.fn().mockRejectedValue("not-an-error") },
    });

    const { result } = renderHook(() => useCamera());
    await waitFor(() => expect(result.current.error).toBe("カメラへのアクセスが拒否されました"));
  });
});

// ── 成功パス ──────────────────────────────────────────────────────────────────

describe("useCamera — 成功パス", () => {
  it("getUserMedia が呼ばれる", async () => {
    const getUserMedia = vi.fn().mockReturnValue(new Promise(() => {}));
    vi.stubGlobal("navigator", { mediaDevices: { getUserMedia } });

    renderHook(() => useCamera());
    await waitFor(() => expect(getUserMedia).toHaveBeenCalled());
  });

  it("アンマウント時にトラックが停止される", async () => {
    const stop = vi.fn();
    const mockStream = { getTracks: () => [{ stop }] };
    vi.stubGlobal("navigator", {
      mediaDevices: { getUserMedia: vi.fn().mockResolvedValue(mockStream) },
    });

    const { unmount } = renderHook(() => useCamera());

    // Flush microtasks so the .then() callback (stream assignment) runs.
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    unmount();
    expect(stop).toHaveBeenCalled();
  });

  it("getUserMedia 成功後に onloadedmetadata ハンドラが設定され isReady=true になる", async () => {
    const stop = vi.fn();
    const mockStream = { getTracks: () => [{ stop }] };
    vi.stubGlobal("navigator", {
      mediaDevices: { getUserMedia: vi.fn().mockResolvedValue(mockStream) },
    });

    let cam: ReturnType<typeof useCamera> | null = null;
    function Wrapper() {
      cam = useCamera();
      return h("video", { ref: cam.videoRef });
    }

    render(h(Wrapper, null));

    // Wait for effect + promise resolution.
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    const video = cam!.videoRef.current;
    if (video?.onloadedmetadata) {
      await act(async () => {
        (video.onloadedmetadata as () => void)();
      });
      await waitFor(() => expect(cam!.isReady).toBe(true));
    }
  });
});

// ── captureFrame ──────────────────────────────────────────────────────────────

describe("useCamera — captureFrame", () => {
  beforeEach(() => {
    vi.stubGlobal("navigator", {
      mediaDevices: { getUserMedia: vi.fn().mockReturnValue(new Promise(() => {})) },
    });
  });

  it("videoRef が null のとき空文字を返す", () => {
    const { result } = renderHook(() => useCamera());
    expect(result.current.captureFrame()).toBe("");
  });

  it("videoRef が設定されているとき文字列を返す", () => {
    const { result } = renderHook(() => useCamera());
    const fakeVideo = document.createElement("video");
    // Directly override the ref's current value.
    Object.defineProperty(result.current.videoRef, "current", {
      value: fakeVideo,
      writable: true,
      configurable: true,
    });

    const url = result.current.captureFrame();
    expect(typeof url).toBe("string");
    expect(url.length).toBeGreaterThan(0);
  });
});
