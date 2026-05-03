import { describe, expect, it, beforeEach } from "vite-plus/test";
import {
  startCountdown,
  beginCapture,
  finishCapture,
  reset,
  setFilter,
  setBeautyParam,
  setMakeupParam,
  setToneParam,
  setBackgroundParam,
  addDekoItem,
  removeDekoItem,
  setProcessedUrl,
  markWasmReady,
  markGpuReady,
  tickCountdown,
} from "~/hooks/useAppState";
import {
  appState,
  capturedPhoto,
  selectedFilter,
  beautyParams,
  makeupParams,
  toneParams,
  backgroundParams,
  dekoItems,
  processedUrl,
  wasmReady,
  gpuReady,
  countdownValue,
} from "~/state/signals";

beforeEach(() => {
  reset();
});

describe("reset", () => {
  it("capturedPhoto を null に戻す", () => {
    capturedPhoto.value = "data:image/png;base64,abc";
    reset();
    expect(capturedPhoto.value).toBeNull();
  });

  it("appState を idle に戻す", () => {
    appState.value = "edit";
    reset();
    expect(appState.value).toBe("idle");
  });

  it("selectedFilter を none に戻す", () => {
    selectedFilter.value = "warm";
    reset();
    expect(selectedFilter.value).toBe("none");
  });

  it("beautyParams をデフォルト値に戻す", () => {
    beautyParams.value = { ...beautyParams.value, skin: false };
    reset();
    expect(beautyParams.value.skin).toBe(true);
    expect(beautyParams.value.slim).toBe(false);
  });

  it("makeupParams をデフォルト値に戻す", () => {
    makeupParams.value = { ...makeupParams.value, lipEnabled: true };
    reset();
    expect(makeupParams.value.lipEnabled).toBe(false);
  });

  it("toneParams をデフォルト値に戻す", () => {
    toneParams.value = { ...toneParams.value, lutPreset: "pop" };
    reset();
    expect(toneParams.value.lutPreset).toBe("none");
  });

  it("backgroundParams をデフォルト値に戻す", () => {
    backgroundParams.value = { ...backgroundParams.value, mode: "blur" };
    reset();
    expect(backgroundParams.value.mode).toBe("none");
  });

  it("dekoItems を空配列に戻す", () => {
    dekoItems.value = [{ id: "a", emoji: "✨", x: 0.5, y: 0.5 }];
    reset();
    expect(dekoItems.value).toHaveLength(0);
  });

  it("processedUrl を null に戻す", () => {
    processedUrl.value = "data:image/png;base64,xyz";
    reset();
    expect(processedUrl.value).toBeNull();
  });
});

describe("startCountdown / beginCapture / finishCapture", () => {
  it("startCountdown で state が countdown になる", () => {
    expect(appState.value).toBe("idle");
    startCountdown();
    expect(appState.value).toBe("countdown");
  });

  it("beginCapture で state が capturing になる", () => {
    startCountdown();
    beginCapture();
    expect(appState.value).toBe("capturing");
  });

  it("finishCapture で state が edit になる", () => {
    startCountdown();
    beginCapture();
    finishCapture();
    expect(appState.value).toBe("edit");
  });
});

describe("setFilter", () => {
  it("フィルターを更新する", () => {
    setFilter("cool");
    expect(selectedFilter.value).toBe("cool");
  });
});

describe("setBeautyParam", () => {
  it("skin フラグを更新する", () => {
    setBeautyParam("skin", false);
    expect(beautyParams.value.skin).toBe(false);
  });

  it("strength を更新する", () => {
    setBeautyParam("strength", 0.42);
    expect(beautyParams.value.strength).toBeCloseTo(0.42);
  });

  it("他のフィールドは変更されない", () => {
    const before = beautyParams.value.blemish;
    setBeautyParam("skin", false);
    expect(beautyParams.value.blemish).toBe(before);
  });
});

describe("setMakeupParam", () => {
  it("lipEnabled を更新する", () => {
    setMakeupParam("lipEnabled", true);
    expect(makeupParams.value.lipEnabled).toBe(true);
  });

  it("lipColor を更新する", () => {
    setMakeupParam("lipColor", "#123456");
    expect(makeupParams.value.lipColor).toBe("#123456");
  });
});

describe("setToneParam", () => {
  it("lutPreset を更新する", () => {
    setToneParam("lutPreset", "film");
    expect(toneParams.value.lutPreset).toBe("film");
  });

  it("overlayEnabled を更新する", () => {
    setToneParam("overlayEnabled", true);
    expect(toneParams.value.overlayEnabled).toBe(true);
  });
});

describe("setBackgroundParam", () => {
  it("mode を更新する", () => {
    setBackgroundParam("mode", "solid");
    expect(backgroundParams.value.mode).toBe("solid");
  });

  it("blurRadius を更新する", () => {
    setBackgroundParam("blurRadius", 20);
    expect(backgroundParams.value.blurRadius).toBe(20);
  });
});

describe("addDekoItem / removeDekoItem", () => {
  it("addDekoItem でリストに追加される", () => {
    addDekoItem({ id: "x1", emoji: "🎉", x: 0.3, y: 0.4 });
    expect(dekoItems.value).toHaveLength(1);
    expect(dekoItems.value[0].emoji).toBe("🎉");
  });

  it("removeDekoItem で指定 id のみ削除される", () => {
    addDekoItem({ id: "x1", emoji: "🎉", x: 0.3, y: 0.4 });
    addDekoItem({ id: "x2", emoji: "✨", x: 0.5, y: 0.5 });
    removeDekoItem("x1");
    expect(dekoItems.value).toHaveLength(1);
    expect(dekoItems.value[0].id).toBe("x2");
  });

  it("存在しない id を削除してもリストは変わらない", () => {
    addDekoItem({ id: "x1", emoji: "🎉", x: 0.3, y: 0.4 });
    removeDekoItem("nonexistent");
    expect(dekoItems.value).toHaveLength(1);
  });
});

describe("setProcessedUrl / markWasmReady / markGpuReady / tickCountdown", () => {
  it("setProcessedUrl で processedUrl が更新される", () => {
    setProcessedUrl("data:image/png;base64,abc");
    expect(processedUrl.value).toBe("data:image/png;base64,abc");
  });

  it("markWasmReady で wasmReady が true になる", () => {
    wasmReady.value = false;
    markWasmReady();
    expect(wasmReady.value).toBe(true);
  });

  it("markGpuReady で gpuReady が true になる", () => {
    gpuReady.value = false;
    markGpuReady();
    expect(gpuReady.value).toBe(true);
  });

  it("tickCountdown で countdownValue が更新される", () => {
    tickCountdown(2);
    expect(countdownValue.value).toBe(2);
  });
});
