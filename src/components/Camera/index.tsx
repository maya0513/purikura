import { useCamera } from "~/hooks/useCamera";
import { runCountdown, delay } from "~/hooks/useCountdown";
import { startCountdown, beginCapture, capturePhoto, finishCapture } from "~/hooks/useAppState";
import { CaptureButton } from "./CaptureButton";
import { CountdownOverlay } from "./CountdownOverlay";

export function CameraView() {
  const { videoRef, isReady, captureFrame, error } = useCamera();

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
    <div class="h-full flex flex-col md:max-w-3xl md:mx-auto md:w-full">
      <div class="flex-1 min-h-0 relative overflow-hidden bg-black md:rounded-2xl md:my-4 md:shadow-lg">
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
        <CountdownOverlay />
      </div>
      <div class="h-20 md:h-24 shrink-0 flex items-center justify-center bg-cream">
        <CaptureButton onClick={handleStartCapture} />
      </div>
    </div>
  );
}
