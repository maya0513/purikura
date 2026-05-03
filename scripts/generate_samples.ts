// Generates sample images using the actual app pipeline in a headless Firefox.
// Face detection (MediaPipe) and segmentation run in the real browser, so
// all landmark-based effects use accurate geometry.
//
// Usage: just samples   (builds web WASM first, then runs this script)
//
// Output layout:
//   photo/original.png
//   photo/filter/{grayscale,sepia,vivid,soft,warm,cool}.png
//   photo/lut/{natural,pop,soft,film,vintage,cool,peach}.png
//   photo/overlay/{pink,multiply,screen,softlight,vignette}.png
//   photo/beauty/{blemish,skin,eyes,slim}.png
//   photo/makeup/{lip,eyeshadow,blush}.png
//   photo/bg/{solid,blur}.png
//   photo/combined.png
//   photo/RESULTS.md

import { firefox, type Browser } from "playwright";
import { spawn, type ChildProcess } from "child_process";
import { writeFileSync, readFileSync, mkdirSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import sharp from "sharp";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");
const OUT = join(ROOT, "photo");
const PORT = 5174;

for (const sub of ["filter", "lut", "overlay", "beauty", "makeup", "bg"]) {
  mkdirSync(join(OUT, sub), { recursive: true });
}

// ── Default params ────────────────────────────────────────────────────────────

const beautyOff = () => ({
  skin: false,
  blemish: false,
  eyes: false,
  slim: false,
  whiten: false,
  eyeSparkle: false,
  strength: 0.8,
  blemishStrength: 0.9,
  eyesStrength: 0.7,
  slimStrength: 0.5,
  whitenStrength: 0.75,
  eyeSparkleStrength: 0.65,
});
const makeupOff = () => ({
  lipEnabled: false,
  lipColor: "#c83248",
  lipStrength: 0.65,
  eyeShadowEnabled: false,
  eyeShadowColor: "#6030b8",
  eyeShadowStrength: 0.7,
  blushEnabled: false,
  blushColor: "#ff6677",
  blushStrength: 0.75,
});
const toneOff = () => ({
  lutPreset: "none" as const,
  overlayEnabled: false,
  overlayColor: "#ffffff",
  overlayStrength: 0.1,
  overlayBlendMode: "normal" as const,
  vignetteStrength: 0,
});
const bgOff = () => ({
  mode: "none" as const,
  blurRadius: 15,
  solidColor: "#ffffff",
  imageDataUrl: null,
});

// ── Vite dev server ───────────────────────────────────────────────────────────

function startVite(): Promise<ChildProcess> {
  return new Promise((resolve, reject) => {
    const proc = spawn("node_modules/.bin/vp", ["dev", "--port", String(PORT)], {
      cwd: ROOT,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const timer = setTimeout(() => reject(new Error("Vite start timeout")), 90_000);
    const check = (data: Buffer) => {
      if (data.toString().includes(`localhost:${PORT}`)) {
        clearTimeout(timer);
        resolve(proc);
      }
    };
    proc.stdout!.on("data", check);
    proc.stderr!.on("data", check);
    proc.on("error", (e) => {
      clearTimeout(timer);
      reject(e);
    });
  });
}

// ── Main ──────────────────────────────────────────────────────────────────────

let vite: ChildProcess | null = null;
let browser: Browser | null = null;

try {
  process.stdout.write("Vite 起動中…\n");
  vite = await startVite();
  process.stdout.write(`Vite ready (port ${PORT})\n`);

  browser = await firefox.launch({ headless: true });
  const page = await browser.newPage();

  // Suppress MediaPipe model console noise
  page.on("console", (msg) => {
    if (msg.type() === "error") process.stderr.write(`[browser] ${msg.text()}\n`);
  });

  // Log all browser console output for debugging
  page.on("console", (msg) => {
    process.stderr.write(`[browser:${msg.type()}] ${msg.text()}\n`);
  });
  page.on("pageerror", (err) => {
    process.stderr.write(`[browser:pageerror] ${err.message}\n`);
  });

  process.stdout.write("ブラウザ起動・WASM / MediaPipe 初期化中…\n");
  await page.goto(`http://localhost:${PORT}/scripts/browser_processor.html`, {
    waitUntil: "domcontentloaded",
    timeout: 60_000,
  });

  // Wait up to 120 s for WASM + MediaPipe face-landmarker model to be ready.
  // NOTE: pass `undefined` as arg so options lands in the correct position
  await page.waitForFunction(
    () => (window as any).__ready === true || !!(window as any).__initError,
    undefined,
    { timeout: 120_000 },
  );

  const initErr = await page.evaluate(() => (window as any).__initError);
  if (initErr) throw new Error(`Init failed in browser: ${initErr}`);
  process.stdout.write("初期化完了\n\n");

  // Read source photo as data URL (JPEG → base64)
  const photoBytes = readFileSync(join(OUT, "myface.jpg"));
  const dataUrl = `data:image/jpeg;base64,${photoBytes.toString("base64")}`;

  // Helper: call processPhoto in the browser, save result to photo/<rel>.png
  async function run(
    rel: string,
    filter: string,
    beauty: object,
    makeup: object,
    tone: object,
    bg: object,
  ): Promise<void> {
    const result = await page.evaluate(
      async ([du, f, be, ma, to, bg]) =>
        window.__processPhoto(
          du as string,
          f as never,
          be as never,
          ma as never,
          to as never,
          bg as never,
        ),
      [dataUrl, filter, beauty, makeup, tone, bg] as const,
    );
    const b64 = (result as string).replace(/^data:image\/\w+;base64,/, "");
    await sharp(Buffer.from(b64, "base64")).toFile(join(OUT, `${rel}.png`));
    process.stdout.write(`  ✓ ${rel}.png\n`);
  }

  // ── original ─────────────────────────────────────────────────────────────
  await run("original", "none", beautyOff(), makeupOff(), toneOff(), bgOff());

  // ── Filters ──────────────────────────────────────────────────────────────
  process.stdout.write("\nフィルター:\n");
  for (const f of ["grayscale", "sepia", "vivid", "soft", "warm", "cool"]) {
    await run(`filter/${f}`, f, beautyOff(), makeupOff(), toneOff(), bgOff());
  }

  // ── LUT presets ───────────────────────────────────────────────────────────
  process.stdout.write("\nLUT プリセット:\n");
  for (const p of ["natural", "pop", "soft", "film", "vintage", "cool", "peach"]) {
    await run(
      `lut/${p}`,
      "none",
      beautyOff(),
      makeupOff(),
      { ...toneOff(), lutPreset: p },
      bgOff(),
    );
  }

  // ── Overlay / vignette ────────────────────────────────────────────────────
  process.stdout.write("\nオーバーレイ・ビネット:\n");
  await run(
    "overlay/pink",
    "none",
    beautyOff(),
    makeupOff(),
    {
      ...toneOff(),
      overlayEnabled: true,
      overlayColor: "#ff69b4",
      overlayStrength: 0.3,
      overlayBlendMode: "normal",
    },
    bgOff(),
  );
  await run(
    "overlay/multiply",
    "none",
    beautyOff(),
    makeupOff(),
    {
      ...toneOff(),
      overlayEnabled: true,
      overlayColor: "#646464",
      overlayStrength: 0.5,
      overlayBlendMode: "multiply",
    },
    bgOff(),
  );
  await run(
    "overlay/screen",
    "none",
    beautyOff(),
    makeupOff(),
    {
      ...toneOff(),
      overlayEnabled: true,
      overlayColor: "#ffffff",
      overlayStrength: 0.4,
      overlayBlendMode: "screen",
    },
    bgOff(),
  );
  await run(
    "overlay/softlight",
    "none",
    beautyOff(),
    makeupOff(),
    {
      ...toneOff(),
      overlayEnabled: true,
      overlayColor: "#808080",
      overlayStrength: 0.6,
      overlayBlendMode: "softlight",
    },
    bgOff(),
  );
  await run(
    "overlay/vignette",
    "none",
    beautyOff(),
    makeupOff(),
    { ...toneOff(), vignetteStrength: 0.75 },
    bgOff(),
  );

  // ── Beauty ────────────────────────────────────────────────────────────────
  process.stdout.write("\n美肌:\n");
  await run(
    "beauty/blemish",
    "none",
    { ...beautyOff(), blemish: true },
    makeupOff(),
    toneOff(),
    bgOff(),
  );
  await run("beauty/skin", "none", { ...beautyOff(), skin: true }, makeupOff(), toneOff(), bgOff());
  await run(
    "beauty/whiten",
    "none",
    { ...beautyOff(), whiten: true, whitenStrength: 0.85 },
    makeupOff(),
    toneOff(),
    bgOff(),
  );
  await run(
    "beauty/eyes",
    "none",
    { ...beautyOff(), eyes: true, eyesStrength: 0.85 },
    makeupOff(),
    toneOff(),
    bgOff(),
  );
  await run(
    "beauty/eye_sparkle",
    "none",
    { ...beautyOff(), eyeSparkle: true, eyeSparkleStrength: 0.8 },
    makeupOff(),
    toneOff(),
    bgOff(),
  );
  await run(
    "beauty/slim",
    "none",
    { ...beautyOff(), slim: true, slimStrength: 0.65 },
    makeupOff(),
    toneOff(),
    bgOff(),
  );

  // ── Makeup ────────────────────────────────────────────────────────────────
  process.stdout.write("\nメイク:\n");
  await run(
    "makeup/lip",
    "none",
    beautyOff(),
    { ...makeupOff(), lipEnabled: true },
    toneOff(),
    bgOff(),
  );
  await run(
    "makeup/eyeshadow",
    "none",
    beautyOff(),
    { ...makeupOff(), eyeShadowEnabled: true },
    toneOff(),
    bgOff(),
  );
  await run(
    "makeup/blush",
    "none",
    beautyOff(),
    { ...makeupOff(), blushEnabled: true },
    toneOff(),
    bgOff(),
  );

  // ── Background ────────────────────────────────────────────────────────────
  process.stdout.write("\n背景:\n");
  await run("bg/solid", "none", beautyOff(), makeupOff(), toneOff(), {
    ...bgOff(),
    mode: "solid",
    solidColor: "#ffd2e0",
  });
  await run("bg/blur", "none", beautyOff(), makeupOff(), toneOff(), {
    ...bgOff(),
    mode: "blur",
    blurRadius: 18,
  });

  // ── Combined ─────────────────────────────────────────────────────────────
  process.stdout.write("\n組み合わせ:\n");
  await run(
    "combined",
    "none",
    {
      ...beautyOff(),
      blemish: true,
      skin: true,
      whiten: true,
      whitenStrength: 0.7,
      eyes: true,
      eyesStrength: 0.6,
      eyeSparkle: true,
      eyeSparkleStrength: 0.7,
    },
    { ...makeupOff(), lipEnabled: true, blushEnabled: true },
    { ...toneOff(), lutPreset: "natural", vignetteStrength: 0.4 },
    bgOff(),
  );

  // ── RESULTS.md ────────────────────────────────────────────────────────────
  const md = `# 画像処理サンプル結果

生成日: ${new Date().toISOString().split("T")[0]}
顔検知: MediaPipe FaceLandmarker (実ブラウザ実行)
背景分離: MediaPipe ImageSegmenter

## 元画像

![original](original.png)

---

## フィルター (apply_filter)

| grayscale | sepia | vivid | soft | warm | cool |
|:---------:|:-----:|:-----:|:----:|:----:|:----:|
| ![](filter/grayscale.png) | ![](filter/sepia.png) | ![](filter/vivid.png) | ![](filter/soft.png) | ![](filter/warm.png) | ![](filter/cool.png) |

---

## LUT プリセット (apply_lut3d)

| natural | pop | soft | film |
|:-------:|:---:|:----:|:----:|
| ![](lut/natural.png) | ![](lut/pop.png) | ![](lut/soft.png) | ![](lut/film.png) |

| vintage | cool | peach |
|:-------:|:----:|:-----:|
| ![](lut/vintage.png) | ![](lut/cool.png) | ![](lut/peach.png) |

---

## カラーオーバーレイ・ビネット (apply_color_overlay)

| pink | multiply | screen | softlight | vignette |
|:----:|:--------:|:------:|:---------:|:--------:|
| ![](overlay/pink.png) | ![](overlay/multiply.png) | ![](overlay/screen.png) | ![](overlay/softlight.png) | ![](overlay/vignette.png) |

---

## 美肌エフェクト (MediaPipe 顔検知使用)

| blemish 除去 | スキン補正 | 白肌 | 目拡大 | 目キラキラ | 小顔 |
|:-----------:|:--------:|:----:|:-----:|:---------:|:---:|
| ![](beauty/blemish.png) | ![](beauty/skin.png) | ![](beauty/whiten.png) | ![](beauty/eyes.png) | ![](beauty/eye_sparkle.png) | ![](beauty/slim.png) |

---

## メイク (MediaPipe 顔パーツ検知使用)

| リップ | アイシャドウ | チーク |
|:-----:|:-----------:|:-----:|
| ![](makeup/lip.png) | ![](makeup/eyeshadow.png) | ![](makeup/blush.png) |

---

## 背景処理 (MediaPipe セグメンテーション使用)

| ソリッド背景 | 背景ぼかし |
|:-----------:|:--------:|
| ![](bg/solid.png) | ![](bg/blur.png) |

---

## 組み合わせ (blemish+skin+whiten+eyes+eyeSparkle → lip+blush → LUT natural → vignette)

![combined](combined.png)
`;

  writeFileSync(join(OUT, "RESULTS.md"), md);
  process.stdout.write("\n✓ photo/RESULTS.md\n");
  process.stdout.write("\n完了！ photo/RESULTS.md で結果を確認してください。\n");
} finally {
  // Playwright's browser.close() can hang; give it 5 s then force-exit.
  await Promise.race([
    browser?.close() ?? Promise.resolve(),
    new Promise<void>((resolve) => setTimeout(resolve, 5_000)),
  ]);
  vite?.kill("SIGKILL");
  process.exit(0);
}
