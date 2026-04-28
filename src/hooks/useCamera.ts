import { useEffect, useRef, useState } from "preact/hooks";
import { PHOTO_WIDTH, PHOTO_HEIGHT } from "~/state/types";

export function useCamera(): {
  videoRef: ReturnType<typeof useRef<HTMLVideoElement>>;
  isReady: boolean;
  captureFrame: () => string;
  error: string | null;
} {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let stream: MediaStream | null = null;

    navigator.mediaDevices
      .getUserMedia({
        video: { width: PHOTO_WIDTH, height: PHOTO_HEIGHT, facingMode: "user" },
        audio: false,
      })
      .then((s) => {
        stream = s;
        if (videoRef.current) {
          videoRef.current.srcObject = s;
          videoRef.current.onloadedmetadata = () => setIsReady(true);
        }
      })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : "カメラへのアクセスが拒否されました");
      });

    return () => {
      stream?.getTracks().forEach((t) => t.stop());
    };
  }, []);

  function captureFrame(): string {
    const video = videoRef.current;
    if (!video) return "";
    const canvas = document.createElement("canvas");
    canvas.width = PHOTO_WIDTH;
    canvas.height = PHOTO_HEIGHT;
    canvas.getContext("2d")!.drawImage(video, 0, 0, PHOTO_WIDTH, PHOTO_HEIGHT);
    return canvas.toDataURL("image/jpeg", 0.92);
  }

  return { videoRef, isReady, captureFrame, error };
}
