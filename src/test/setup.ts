import { vi } from "vite-plus/test";

vi.mock("~/wasm/pkg/purikura_wasm", () => ({
  default: vi.fn().mockResolvedValue({}),
  apply_filter: vi.fn((pixels: Uint8Array) => pixels),
  compose_frame: vi.fn((photo: Uint8Array) => photo),
  create_strip: vi.fn((photos: Uint8Array) => photos),
  build_skin_mask: vi.fn((pixels: Uint8Array, w: number, h: number) => new Uint8Array(w * h)),
  apply_beauty: vi.fn((pixels: Uint8Array) => pixels),
  remove_blemish: vi.fn((pixels: Uint8Array) => pixels),
  enlarge_eyes: vi.fn((pixels: Uint8Array) => pixels),
}));
