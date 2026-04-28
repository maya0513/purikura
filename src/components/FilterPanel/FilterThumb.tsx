import type { FilterName } from "~/state/types";

const FILTER_LABELS: Record<FilterName, string> = {
  none: "なし",
  grayscale: "モノクロ",
  sepia: "セピア",
  vivid: "ビビッド",
  soft: "ソフト",
  warm: "ウォーム",
  cool: "クール",
};

interface Props {
  name: FilterName;
  previewUrl: string;
  isSelected: boolean;
  onSelect: () => void;
}

export function FilterThumb({ name, previewUrl, isSelected, onSelect }: Props) {
  return (
    <button
      class={`flex flex-col items-center gap-1 p-2 rounded-xl transition-all ${
        isSelected ? "ring-2 ring-bubblegum bg-candy-pink/30" : "hover:bg-gray-100"
      }`}
      onClick={onSelect}
    >
      <img src={previewUrl} alt={FILTER_LABELS[name]} class="w-16 h-16 rounded-lg object-cover" />
      <span class="text-xs font-medium">{FILTER_LABELS[name]}</span>
    </button>
  );
}
