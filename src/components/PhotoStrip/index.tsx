import { useEffect, useRef, useState } from "preact/hooks";
import {
  capturedPhoto,
  selectedFilter,
  beautyParams,
  makeupParams,
  toneParams,
  backgroundParams,
  dekoItems,
  processedUrl,
} from "~/state/signals";
import { setProcessedUrl, addDekoItem, removeDekoItem, reset } from "~/hooks/useAppState";
import { processPhoto, renderFinal } from "~/lib/imageProcessor";
import { PHOTO_WIDTH, PHOTO_HEIGHT } from "~/state/types";
import { BeautyPanel } from "~/components/BeautyPanel";
import { MakeupPanel } from "~/components/MakeupPanel";
import { TonePanel } from "~/components/TonePanel";
import { BackgroundPanel } from "~/components/BackgroundPanel";
import { StickerPanel } from "~/components/StickerPanel";
import type { DekoItem } from "~/state/types";

type Tab = "beauty" | "makeup" | "tone" | "background" | "sticker";

const TABS: { id: Tab; label: string; emoji: string }[] = [
  { id: "beauty", label: "美肌", emoji: "✨" },
  { id: "makeup", label: "メイク", emoji: "💋" },
  { id: "tone", label: "トーン", emoji: "🌈" },
  { id: "background", label: "背景", emoji: "🖼️" },
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
  const beauty = beautyParams.value;
  const makeup = makeupParams.value;
  const tone = toneParams.value;
  const background = backgroundParams.value;
  const items = dekoItems.value;
  const url = processedUrl.value;
  const [selectedEmoji, setSelectedEmoji] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("beauty");
  const containerRef = useRef<HTMLDivElement>(null);
  const [imageBounds, setImageBounds] = useState<ImageBounds | null>(null);

  useEffect(() => {
    if (!photo) return;
    let cancelled = false;
    processPhoto(photo, filter, beauty, makeup, tone, background).then((result) => {
      if (!cancelled) setProcessedUrl(result);
    });
    return () => {
      cancelled = true;
    };
  }, [
    photo,
    filter,
    beauty.skin,
    beauty.blemish,
    beauty.eyes,
    beauty.slim,
    beauty.strength,
    beauty.blemishStrength,
    beauty.eyesStrength,
    beauty.slimStrength,
    makeup.lipEnabled,
    makeup.lipColor,
    makeup.lipStrength,
    makeup.eyeShadowEnabled,
    makeup.eyeShadowColor,
    makeup.eyeShadowStrength,
    makeup.blushEnabled,
    makeup.blushColor,
    makeup.blushStrength,
    tone.lutPreset,
    tone.overlayEnabled,
    tone.overlayColor,
    tone.overlayStrength,
    tone.overlayBlendMode,
    tone.vignetteStrength,
    background.mode,
    background.blurRadius,
    background.solidColor,
    background.imageDataUrl,
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
    <div class="h-full flex flex-col md:flex-row md:max-w-6xl md:mx-auto md:w-full">
      {/* Photo area */}
      <div
        ref={containerRef}
        class={`flex-1 min-h-0 min-w-0 relative overflow-hidden bg-black md:rounded-2xl md:my-4 md:ml-4 md:shadow-lg ${selectedEmoji ? "cursor-crosshair" : "cursor-default"}`}
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

      {/* Sidebar */}
      <div class="flex flex-col md:w-80 md:shrink-0 md:my-4 md:ml-4 md:mr-4">
        {/* Tab bar */}
        <div class="h-11 shrink-0 flex border-t border-candy-pink/30 bg-white md:border-t-0 md:rounded-t-2xl md:overflow-hidden">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              class={`flex-1 flex flex-col items-center justify-center gap-0.5 text-xs transition-colors ${
                activeTab === tab.id
                  ? "text-bubblegum border-t-2 border-bubblegum bg-candy-pink/10 md:border-t-0 md:border-b-2 md:border-bubblegum"
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
        <div class="h-32 shrink-0 bg-white/90 border-t border-candy-pink/20 px-3 py-2 md:h-auto md:flex-1 md:min-h-0 md:border-t md:border-candy-pink/30">
          {activeTab === "beauty" && <BeautyPanel />}
          {activeTab === "makeup" && <MakeupPanel />}
          {activeTab === "tone" && <TonePanel />}
          {activeTab === "background" && <BackgroundPanel />}
          {activeTab === "sticker" && (
            <StickerPanel selected={selectedEmoji} onSelect={setSelectedEmoji} />
          )}
        </div>

        {/* Actions */}
        <div class="h-14 shrink-0 flex gap-3 px-4 items-center justify-center border-t border-candy-pink/30 bg-white md:rounded-b-2xl md:py-3">
          <button
            class="btn-primary py-2 px-5 text-sm disabled:opacity-40 disabled:cursor-not-allowed"
            onClick={handleDownload}
            disabled={!url}
          >
            ⬇️ ダウンロード
          </button>
          <button
            class="btn-primary bg-lavender text-soft-purple py-2 px-5 text-sm"
            onClick={reset}
          >
            🔄 やり直す
          </button>
        </div>
      </div>
    </div>
  );
}
