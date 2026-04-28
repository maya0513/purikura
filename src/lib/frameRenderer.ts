import { PHOTO_WIDTH, PHOTO_HEIGHT } from "~/state/types";
import type { FrameName } from "~/state/types";

type DecorativeFrame = Exclude<FrameName, "none">;

const FRAME_EMOJIS: Record<DecorativeFrame, string[]> = {
  hearts: ["💕", "💖", "💗", "💞"],
  stars: ["⭐", "✨", "🌟", "💫"],
  flowers: ["🌸", "🌼", "🌺", "🌷"],
  bubbles: ["🫧", "💧", "🔵", "⚪"],
};

const cache = new Map<DecorativeFrame, Uint8ClampedArray>();

function buildPositions(): [number, number][] {
  const positions: [number, number][] = [];
  const step = 88;
  const inset = 40;

  for (let x = inset; x <= PHOTO_WIDTH - inset; x += step) {
    positions.push([x, inset]);
    positions.push([x, PHOTO_HEIGHT - inset]);
  }
  for (let y = inset + step; y <= PHOTO_HEIGHT - inset - step; y += step) {
    positions.push([inset, y]);
    positions.push([PHOTO_WIDTH - inset, y]);
  }
  return positions;
}

export function renderFrame(name: DecorativeFrame): Uint8ClampedArray {
  const cached = cache.get(name);
  if (cached) return cached;

  const canvas = document.createElement("canvas");
  canvas.width = PHOTO_WIDTH;
  canvas.height = PHOTO_HEIGHT;
  const ctx = canvas.getContext("2d")!;

  const emojis = FRAME_EMOJIS[name];
  ctx.font = "56px serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";

  buildPositions().forEach(([x, y], i) => {
    ctx.fillText(emojis[i % emojis.length], x, y);
  });

  const data = ctx.getImageData(0, 0, PHOTO_WIDTH, PHOTO_HEIGHT).data;
  cache.set(name, data);
  return data;
}
