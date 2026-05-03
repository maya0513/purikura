import { describe, expect, it } from "vite-plus/test";
import { hexToRgb } from "~/state/types";

describe("hexToRgb", () => {
  it("赤を正しく変換する", () => {
    expect(hexToRgb("#ff0000")).toEqual([255, 0, 0]);
  });

  it("黒を正しく変換する", () => {
    expect(hexToRgb("#000000")).toEqual([0, 0, 0]);
  });

  it("白を正しく変換する", () => {
    expect(hexToRgb("#ffffff")).toEqual([255, 255, 255]);
  });

  it("任意の色を正しく変換する", () => {
    expect(hexToRgb("#1a2b3c")).toEqual([26, 43, 60]);
  });

  it("大文字でも正しく変換する", () => {
    expect(hexToRgb("#FF8800")).toEqual([255, 136, 0]);
  });
});
