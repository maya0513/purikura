import { useEffect, useRef, useState } from "preact/hooks";
import {
  capturedPhoto,
  selectedFilter,
  selectedFrame,
  beautyParams,
  dekoItems,
  processedUrl,
} from "~/state/signals";
import { setProcessedUrl, addDekoItem, removeDekoItem, reset } from "~/hooks/useAppState";
import { processPhoto, renderFinal } from "~/lib/imageProcessor";
import { PHOTO_WIDTH, PHOTO_HEIGHT } from "~/state/types";
import { FilterPanel } from "~/components/FilterPanel";
import { FramePanel } from "~/components/FramePanel";
import { BeautyPanel } from "~/components/BeautyPanel";
import { StickerPanel } from "~/components/StickerPanel";
import type { DekoItem } from "~/state/types";

type Tab = "beauty" | "filter" | "frame" | "sticker";

const TABS: { id: Tab; label: string; emoji: string }[] = [
  { id: "beauty", label: "美容", emoji: "💄" },
  { id: "filter", label: "フィルター", emoji: "🎞️" },
  { id: "frame", label: "フレーム", emoji: "🖼️" },
  { id: "sticker", label: "スタンプ", emoji: "🎨" },
];

type ImageBounds = { left: number; top: number; width: number; height: number };

function computeImageBounds(el: HTMLElement): ImageBounds {
  const w = el.offsetWidth;
  const h = el.offsetHeight;
  const imgAspect = PHOTO_WIDTH / PHOTO_HEIGHT;
  const containerAspect = w / h;
  if (containerAspect >= imgAspect) {
    const imgH = h;
    const imgW = imgH * imgAspect;
    return { left: (w - imgW) / 2, top: 0, width: imgW, height: imgH };
  }
  const imgW = w;
  const imgH = imgW / imgAspect;
  return { left: 0, top: (h - imgH) / 2, width: imgW, height: imgH };
}

export function EditView() {
  const photo = capturedPhoto.value;
  const filter = selectedFilter.value;
  const frame = selectedFrame.value;
  const beauty = beautyParams.value;
  const items = dekoItems.value;
  const url = processedUrl.value;
  const [selectedEmoji, setSelectedEmoji] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("beauty");
  const containerRef = useRef<HTMLDivElement>(null);
  const [imageBounds, setImageBounds] = useState<ImageBounds | null>(null);

  useEffect(() => {
    if (!photo) return;
    let cancelled = false;
    processPhoto(photo, filter, frame, beauty).then((result) => {
      if (!cancelled) setProcessedUrl(result);
    });
    return () => {
      cancelled = true;
    };
  }, [
    photo,
    filter,
    frame,
    beauty.skin,
    beauty.blemish,
    beauty.eyes,
    beauty.strength,
    beauty.blemishStrength,
    beauty.eyesStrength,
  ]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el || !url) return;
    const update = () => setImageBounds(computeImageBounds(el));
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, [url]);

  function handlePhotoClick(e: MouseEvent) {
    if (!selectedEmoji || !imageBounds || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const cx = e.clientX - rect.left - imageBounds.left;
    const cy = e.clientY - rect.top - imageBounds.top;
    if (cx < 0 || cy < 0 || cx > imageBounds.width || cy > imageBounds.height) return;
    const item: DekoItem = {
      id: crypto.randomUUID(),
      emoji: selectedEmoji,
      x: cx / imageBounds.width,
      y: cy / imageBounds.height,
    };
    addDekoItem(item);
  }

  async function handleDownload() {
    if (!url) return;
    const final = await renderFinal(url, items);
    const a = document.createElement("a");
    a.href = final;
    a.download = "purikura.png";
    a.click();
  }

  return (
    <div class="h-full flex flex-col">
      {/* Photo area */}
      <div
        ref={containerRef}
        class={`flex-1 min-h-0 relative overflow-hidden bg-black ${selectedEmoji ? "cursor-crosshair" : "cursor-default"}`}
        onClick={handlePhotoClick}
      >
        {url ? (
          <img src={url} alt="プリクラ" class="absolute inset-0 w-full h-full object-contain" />
        ) : (
          <div class="absolute inset-0 flex items-center justify-center">
            <p class="text-white/70 animate-pulse">処理中...</p>
          </div>
        )}
        {imageBounds && (
          <div
            class="absolute pointer-events-none"
            style={{
              left: imageBounds.left,
              top: imageBounds.top,
              width: imageBounds.width,
              height: imageBounds.height,
            }}
          >
            {items.map((item) => (
              <span
                key={item.id}
                class="absolute pointer-events-auto text-4xl select-none cursor-pointer hover:scale-125 transition-transform"
                style={{
                  left: `${item.x * 100}%`,
                  top: `${item.y * 100}%`,
                  transform: "translate(-50%, -50%)",
                  lineHeight: 1,
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  removeDekoItem(item.id);
                }}
              >
                {item.emoji}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Tab bar */}
      <div class="h-11 shrink-0 flex border-t border-candy-pink/30 bg-white">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            class={`flex-1 flex flex-col items-center justify-center gap-0.5 text-xs transition-colors ${
              activeTab === tab.id
                ? "text-bubblegum border-t-2 border-bubblegum bg-candy-pink/10"
                : "text-gray-400 hover:text-gray-600"
            }`}
            onClick={() => setActiveTab(tab.id)}
          >
            <span class="text-base leading-none">{tab.emoji}</span>
            <span class="leading-none">{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Panel content */}
      <div class="h-28 shrink-0 bg-white/90 border-t border-candy-pink/20 px-3 py-2">
        {activeTab === "beauty" && <BeautyPanel />}
        {activeTab === "filter" && <FilterPanel />}
        {activeTab === "frame" && <FramePanel />}
        {activeTab === "sticker" && (
          <StickerPanel selected={selectedEmoji} onSelect={setSelectedEmoji} />
        )}
      </div>

      {/* Actions */}
      <div class="h-14 shrink-0 flex gap-3 px-4 items-center justify-center border-t border-candy-pink/30 bg-white">
        <button
          class="btn-primary py-2 px-5 text-sm disabled:opacity-40 disabled:cursor-not-allowed"
          onClick={handleDownload}
          disabled={!url}
        >
          ⬇️ ダウンロード
        </button>
        <button class="btn-primary bg-lavender text-soft-purple py-2 px-5 text-sm" onClick={reset}>
          🔄 やり直す
        </button>
      </div>
    </div>
  );
}
