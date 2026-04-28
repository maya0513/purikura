import { useRef } from "preact/hooks";
import { useCamera } from "~/hooks/useCamera";
import { runCountdown, delay } from "~/hooks/useCountdown";
import { startCountdown, beginCapture, capturePhoto, finishCapture } from "~/hooks/useAppState";
import { useFaceFrame } from "~/hooks/useFaceFrame";
import { appState } from "~/state/signals";
import { CaptureButton } from "./CaptureButton";
import { CountdownOverlay } from "./CountdownOverlay";

export function CameraView() {
  const { videoRef, isReady, captureFrame, error } = useCamera();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const state = appState.value;
  const isIdle = state === "idle";

  useFaceFrame(videoRef, canvasRef, isIdle && isReady);

  async function handleStartCapture() {
    startCountdown();
    await runCountdown(3);
    beginCapture();
    capturePhoto(captureFrame());
    await delay(400);
    finishCapture();
  }

  if (error) {
    return (
      <div class="h-full flex items-center justify-center p-8">
        <div class="kawaii-card text-center">
          <p class="text-red-500 text-lg">⚠️ {error}</p>
          <p class="text-sm text-gray-500 mt-2">カメラのアクセスを許可してください</p>
        </div>
      </div>
    );
  }

  return (
    <div class="h-full flex flex-col">
      <div class="flex-1 min-h-0 relative overflow-hidden bg-black">
        <video
          ref={videoRef}
          autoPlay
          playsInline
          muted
          class="absolute inset-0 w-full h-full object-cover"
          style={{ transform: "scaleX(-1)" }}
        />
        {!isReady && (
          <div class="absolute inset-0 flex items-center justify-center bg-lavender">
            <p class="text-soft-purple text-lg animate-pulse">カメラ起動中...</p>
          </div>
        )}
        <canvas ref={canvasRef} class="absolute inset-0 w-full h-full pointer-events-none" />
        <CountdownOverlay />
      </div>
      <div class="h-20 shrink-0 flex items-center justify-center bg-cream">
        <CaptureButton onClick={handleStartCapture} />
      </div>
    </div>
  );
}
