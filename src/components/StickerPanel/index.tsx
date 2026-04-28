const STICKERS = [
  "❤️",
  "🩷",
  "🧡",
  "💛",
  "💚",
  "💙",
  "💜",
  "🤍",
  "⭐",
  "🌟",
  "✨",
  "💫",
  "🎀",
  "👑",
  "💎",
  "🎵",
  "🌸",
  "🌺",
  "🌻",
  "🦋",
  "🌈",
  "🍭",
  "🌙",
  "🎈",
  "🐱",
  "🐰",
  "🍓",
  "🎂",
  "☁️",
  "🎉",
];

interface Props {
  selected: string | null;
  onSelect: (emoji: string | null) => void;
}

export function StickerPanel({ selected, onSelect }: Props) {
  return (
    <div class="flex flex-col gap-1 h-full">
      <div class="flex items-center justify-between shrink-0">
        <p class="text-xs text-gray-400">選んで写真をタップ</p>
        {selected && (
          <button class="text-xs text-bubblegum font-medium" onClick={() => onSelect(null)}>
            ✕ 解除
          </button>
        )}
      </div>
      <div class="flex gap-1 overflow-x-auto flex-1 items-center">
        {STICKERS.map((s) => (
          <button
            key={s}
            class={`text-3xl p-1 rounded-lg shrink-0 transition-all ${
              selected === s
                ? "bg-candy-pink/40 ring-2 ring-bubblegum scale-110"
                : "hover:bg-lavender hover:scale-105"
            }`}
            onClick={() => onSelect(selected === s ? null : s)}
          >
            {s}
          </button>
        ))}
      </div>
    </div>
  );
}
