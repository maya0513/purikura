import { selectedFrame } from "~/state/signals";
import { setFrame } from "~/hooks/useAppState";
import type { FrameName } from "~/state/types";
import { FrameThumb } from "./FrameThumb";

const FRAMES: FrameName[] = ["none", "hearts", "stars", "flowers", "bubbles"];

export function FramePanel() {
  const current = selectedFrame.value;

  return (
    <div class="flex gap-2 overflow-x-auto h-full items-center pb-1">
      {FRAMES.map((f) => (
        <FrameThumb key={f} name={f} isSelected={current === f} onSelect={() => setFrame(f)} />
      ))}
    </div>
  );
}
