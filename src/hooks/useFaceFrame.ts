import { useEffect, useRef } from "preact/hooks";

type FaceBox = { x: number; y: number; width: number; height: number };

const FRAME_DECORATIONS = ["💕", "⭐", "✨", "🌸"];

function drawFaceFrame(ctx: CanvasRenderingContext2D, box: FaceBox, _w: number, _h: number) {
  const { x, y, width, height } = box;
  const pad = Math.min(width, height) * 0.12;
  const fx = x - pad;
  const fy = y - pad;
  const fw = width + pad * 2;
  const fh = height + pad * 2;

  // Rounded rect border
  ctx.save();
  ctx.strokeStyle = "#ff69b4";
  ctx.lineWidth = 3;
  ctx.shadowColor = "#ff69b4";
  ctx.shadowBlur = 8;
  const r = 16;
  ctx.beginPath();
  ctx.moveTo(fx + r, fy);
  ctx.lineTo(fx + fw - r, fy);
  ctx.arcTo(fx + fw, fy, fx + fw, fy + r, r);
  ctx.lineTo(fx + fw, fy + fh - r);
  ctx.arcTo(fx + fw, fy + fh, fx + fw - r, fy + fh, r);
  ctx.lineTo(fx + r, fy + fh);
  ctx.arcTo(fx, fy + fh, fx, fy + fh - r, r);
  ctx.lineTo(fx, fy + r);
  ctx.arcTo(fx, fy, fx + r, fy, r);
  ctx.closePath();
  ctx.stroke();
  ctx.restore();

  // Corner decorations
  const corners = [
    [fx, fy],
    [fx + fw, fy],
    [fx, fy + fh],
    [fx + fw, fy + fh],
  ] as [number, number][];

  ctx.font = `${Math.max(18, Math.min(28, fw * 0.1))}px serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  corners.forEach(([cx, cy], i) => {
    ctx.fillText(FRAME_DECORATIONS[i], cx, cy);
  });

  // "撮影準備OK!" label
  ctx.save();
  ctx.font = `bold ${Math.max(12, Math.min(16, fw * 0.07))}px sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "bottom";
  ctx.fillStyle = "#ff69b4";
  ctx.shadowColor = "white";
  ctx.shadowBlur = 4;
  ctx.fillText("撮影準備 OK! ✨", fx + fw / 2, fy - 4);
  ctx.restore();
}

export function useFaceFrame(
  videoRef: ReturnType<typeof useRef<HTMLVideoElement>>,
  canvasRef: ReturnType<typeof useRef<HTMLCanvasElement>>,
  active: boolean,
) {
  const rafRef = useRef<number>(0);
  const detectorRef = useRef<any>(null);

  useEffect(() => {
    if (!active) {
      cancelAnimationFrame(rafRef.current);
      const canvas = canvasRef.current;
      if (canvas) {
        const ctx = canvas.getContext("2d");
        ctx?.clearRect(0, 0, canvas.width, canvas.height);
      }
      return;
    }

    if (!("FaceDetector" in window)) return;

    async function init() {
      detectorRef.current = new (window as any).FaceDetector({
        fastMode: true,
        maxDetectedFaces: 1,
      });
      loop();
    }

    async function loop() {
      const video = videoRef.current;
      const canvas = canvasRef.current;
      if (!video || !canvas || video.readyState < 2) {
        rafRef.current = requestAnimationFrame(loop);
        return;
      }

      canvas.width = video.videoWidth || video.clientWidth;
      canvas.height = video.videoHeight || video.clientHeight;
      const ctx = canvas.getContext("2d")!;
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      try {
        const faces = await detectorRef.current.detect(video);
        if (faces.length > 0) {
          const bb = faces[0].boundingBox;
          // Mirror X coordinate since video is flipped
          const mirroredX = canvas.width - bb.left - bb.width;
          drawFaceFrame(
            ctx,
            { x: mirroredX, y: bb.top, width: bb.width, height: bb.height },
            canvas.width,
            canvas.height,
          );
        }
      } catch {
        // FaceDetector may fail on some frames — silently continue
      }

      rafRef.current = requestAnimationFrame(loop);
    }

    init();
    return () => cancelAnimationFrame(rafRef.current);
  }, [active]);
}
