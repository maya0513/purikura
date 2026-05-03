import { vi } from "vite-plus/test";

vi.mock("~/wasm/pkg/purikura_wasm", () => ({
  default: vi.fn().mockResolvedValue({}),
  apply_filter: vi.fn((pixels: Uint8Array) => pixels),
  build_skin_mask: vi.fn((pixels: Uint8Array, w: number, h: number) => new Uint8Array(w * h)),
  apply_beauty: vi.fn((pixels: Uint8Array) => pixels),
  remove_blemish: vi.fn((pixels: Uint8Array) => pixels),
  enlarge_eyes: vi.fn((pixels: Uint8Array) => pixels),
  slim_face: vi.fn((pixels: Uint8Array) => pixels),
  whiten_skin: vi.fn((pixels: Uint8Array) => pixels),
  apply_eye_sparkle: vi.fn((pixels: Uint8Array) => pixels),
  apply_makeup: vi.fn((pixels: Uint8Array) => pixels),
  apply_color_overlay: vi.fn((pixels: Uint8Array) => pixels),
  init_gpu: vi.fn().mockResolvedValue(undefined),
  apply_lut3d: vi.fn((pixels: Uint8Array) => Promise.resolve(pixels)),
  process_background: vi.fn((pixels: Uint8Array) => Promise.resolve(pixels)),
}));
