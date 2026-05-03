import { describe, expect, it, vi, beforeAll, beforeEach } from "vite-plus/test";

// ImageData is not available in happy-dom — stub it.
beforeAll(() => {
  vi.stubGlobal(
    "ImageData",
    class {
      data: Uint8ClampedArray;
      width: number;
      height: number;
      constructor(data: Uint8ClampedArray, w: number, h: number) {
        this.data = data;
        this.width = w;
        this.height = h;
      }
    },
  );

  // Canvas stub: putImageData is a no-op.
  const origCreate = document.createElement.bind(document);
  vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
    if (tag === "canvas")
      return {
        width: 0,
        height: 0,
        getContext: () => ({ putImageData: vi.fn() }),
      } as unknown as HTMLCanvasElement;
    return origCreate(tag as "div");
  });
});

// Reset module registry so each test gets a fresh `segmenterPromise = null`.
beforeEach(() => {
  vi.resetModules();
});

function makeRgba(w = 4, h = 4) {
  return new Uint8ClampedArray(w * h * 4).fill(128);
}

function makeMaskResult(masks: unknown[]) {
  return { confidenceMasks: masks };
}

describe("extractSegmentationMask", () => {
  it("1 マスクが返ったとき foreground マスクデータを返す", async () => {
    const mockData = new Float32Array([0.9, 0.8, 0.7]);
    const mockClose = vi.fn();
    const mockSegment = vi
      .fn()
      .mockReturnValue(makeMaskResult([{ getAsFloat32Array: () => mockData, close: mockClose }]));
    const mockCreate = vi.fn().mockResolvedValue({ segment: mockSegment });

    vi.doMock("@mediapipe/tasks-vision", () => ({
      FilesetResolver: { forVisionTasks: vi.fn().mockResolvedValue({}) },
      ImageSegmenter: { createFromOptions: mockCreate },
    }));

    const { extractSegmentationMask } = await import("~/lib/segmentation");
    const result = await extractSegmentationMask(makeRgba(), 4, 4);
    expect(result).toBeInstanceOf(Float32Array);
    expect(mockClose).toHaveBeenCalled();
  });

  it("2 マスクが返ったとき masks[1] (前景) を使う", async () => {
    const bgData = new Float32Array([0.1]);
    const fgData = new Float32Array([0.9]);
    const mockSegment = vi.fn().mockReturnValue(
      makeMaskResult([
        { getAsFloat32Array: () => bgData, close: vi.fn() },
        { getAsFloat32Array: () => fgData, close: vi.fn() },
      ]),
    );
    const mockCreate = vi.fn().mockResolvedValue({ segment: mockSegment });

    vi.doMock("@mediapipe/tasks-vision", () => ({
      FilesetResolver: { forVisionTasks: vi.fn().mockResolvedValue({}) },
      ImageSegmenter: { createFromOptions: mockCreate },
    }));

    const { extractSegmentationMask } = await import("~/lib/segmentation");
    const result = await extractSegmentationMask(makeRgba(), 4, 4);
    expect(result).toStrictEqual(fgData);
  });

  it("masks が空のとき null を返す", async () => {
    const mockSegment = vi.fn().mockReturnValue(makeMaskResult([]));
    const mockCreate = vi.fn().mockResolvedValue({ segment: mockSegment });

    vi.doMock("@mediapipe/tasks-vision", () => ({
      FilesetResolver: { forVisionTasks: vi.fn().mockResolvedValue({}) },
      ImageSegmenter: { createFromOptions: mockCreate },
    }));

    const { extractSegmentationMask } = await import("~/lib/segmentation");
    const result = await extractSegmentationMask(makeRgba(), 4, 4);
    expect(result).toBeNull();
  });

  it("masks が null のとき null を返す", async () => {
    const mockSegment = vi.fn().mockReturnValue({ confidenceMasks: null });
    const mockCreate = vi.fn().mockResolvedValue({ segment: mockSegment });

    vi.doMock("@mediapipe/tasks-vision", () => ({
      FilesetResolver: { forVisionTasks: vi.fn().mockResolvedValue({}) },
      ImageSegmenter: { createFromOptions: mockCreate },
    }));

    const { extractSegmentationMask } = await import("~/lib/segmentation");
    const result = await extractSegmentationMask(makeRgba(), 4, 4);
    expect(result).toBeNull();
  });

  it("例外が throw されたとき null を返す", async () => {
    const mockSegment = vi.fn().mockImplementation(() => {
      throw new Error("GPU error");
    });
    const mockCreate = vi.fn().mockResolvedValue({ segment: mockSegment });

    vi.doMock("@mediapipe/tasks-vision", () => ({
      FilesetResolver: { forVisionTasks: vi.fn().mockResolvedValue({}) },
      ImageSegmenter: { createFromOptions: mockCreate },
    }));

    const { extractSegmentationMask } = await import("~/lib/segmentation");
    const result = await extractSegmentationMask(makeRgba(), 4, 4);
    expect(result).toBeNull();
  });

  it("loadSegmenter がキャッシュされる（2 回目は createFromOptions を呼ばない）", async () => {
    const mockData = new Float32Array([0.5]);
    const mockSegment = vi
      .fn()
      .mockReturnValue(makeMaskResult([{ getAsFloat32Array: () => mockData, close: vi.fn() }]));
    const mockCreate = vi.fn().mockResolvedValue({ segment: mockSegment });

    vi.doMock("@mediapipe/tasks-vision", () => ({
      FilesetResolver: { forVisionTasks: vi.fn().mockResolvedValue({}) },
      ImageSegmenter: { createFromOptions: mockCreate },
    }));

    const { extractSegmentationMask } = await import("~/lib/segmentation");
    await extractSegmentationMask(makeRgba(), 4, 4);
    await extractSegmentationMask(makeRgba(), 4, 4);
    expect(mockCreate).toHaveBeenCalledOnce();
  });
});
