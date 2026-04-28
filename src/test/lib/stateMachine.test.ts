import { describe, it, expect } from "vite-plus/test";
import {
  nextStateOnStart,
  nextStateOnCapture,
  nextStateOnFinish,
  nextStateOnReset,
  canStartCapture,
  canFinishCapture,
} from "~/lib/stateMachine";

describe("stateMachine", () => {
  describe("nextStateOnStart", () => {
    it("idle → countdown", () => {
      expect(nextStateOnStart("idle")).toBe("countdown");
    });
    it("非idle では変化しない", () => {
      expect(nextStateOnStart("countdown")).toBe("countdown");
      expect(nextStateOnStart("capturing")).toBe("capturing");
    });
  });

  describe("nextStateOnCapture", () => {
    it("countdown → capturing", () => {
      expect(nextStateOnCapture("countdown")).toBe("capturing");
    });
    it("非countdown では変化しない", () => {
      expect(nextStateOnCapture("idle")).toBe("idle");
    });
  });

  describe("nextStateOnFinish", () => {
    it("capturing → edit", () => {
      expect(nextStateOnFinish("capturing")).toBe("edit");
    });
    it("非capturing では変化しない", () => {
      expect(nextStateOnFinish("idle")).toBe("idle");
    });
  });

  describe("nextStateOnReset", () => {
    it("常にidleに戻る", () => {
      expect(nextStateOnReset("edit")).toBe("idle");
      expect(nextStateOnReset("capturing")).toBe("idle");
      expect(nextStateOnReset("idle")).toBe("idle");
    });
  });

  describe("canStartCapture", () => {
    it("idle + wasmReady = true", () => {
      expect(canStartCapture("idle", true)).toBe(true);
    });
    it("idle + wasm未ready = false", () => {
      expect(canStartCapture("idle", false)).toBe(false);
    });
    it("非idle = false", () => {
      expect(canStartCapture("countdown", true)).toBe(false);
    });
  });

  describe("canFinishCapture", () => {
    it("1枚で true", () => {
      expect(canFinishCapture(1)).toBe(true);
    });
    it("0枚で false", () => {
      expect(canFinishCapture(0)).toBe(false);
    });
  });
});
