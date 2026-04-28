import type { FrameName } from "~/state/types";

const FRAME_LABELS: Record<FrameName, string> = {
  none: "なし",
  hearts: "ハート",
  stars: "スター",
  flowers: "フラワー",
  bubbles: "バブル",
};

const FRAME_EMOJI: Record<FrameName, string> = {
  none: "🚫",
  hearts: "💕",
  stars: "⭐",
  flowers: "🌸",
  bubbles: "🫧",
};

interface Props {
  name: FrameName;
  isSelected: boolean;
  onSelect: () => void;
}

export function FrameThumb({ name, isSelected, onSelect }: Props) {
  return (
    <button
      class={`flex flex-col items-center gap-1 p-3 rounded-xl transition-all min-w-16 ${
        isSelected ? "ring-2 ring-bubblegum bg-candy-pink/30" : "hover:bg-gray-100"
      }`}
      onClick={onSelect}
    >
      <span class="text-3xl">{FRAME_EMOJI[name]}</span>
      <span class="text-xs font-medium">{FRAME_LABELS[name]}</span>
    </button>
  );
}
