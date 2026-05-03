// MediaPipe ImageSegmenter wrapper for selfie segmentation.
//
// Returns a per-pixel foreground probability mask (Float32Array, values 0..1
// where 1 = foreground/person, 0 = background).  Lazy-loaded from CDN so the
// ~10 MB model is only fetched when background processing is actually requested.

import { ImageSegmenter, FilesetResolver } from "@mediapipe/tasks-vision";

const MP_BASE = import.meta.env.DEV
  ? "/mediapipe-wasm"
  : "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.22/wasm";
const SEGMENTER_MODEL_URL =
  "https://storage.googleapis.com/mediapipe-models/image_segmenter/selfie_segmenter_landscape/float16/latest/selfie_segmenter_landscape.tflite";

let segmenterPromise: Promise<ImageSegmenter> | null = null;

async function loadSegmenter(): Promise<ImageSegmenter> {
  if (segmenterPromise) return segmenterPromise;
  segmenterPromise = (async () => {
    const vision = await FilesetResolver.forVisionTasks(MP_BASE);
    return ImageSegmenter.createFromOptions(vision, {
      baseOptions: { modelAssetPath: SEGMENTER_MODEL_URL, delegate: "GPU" },
      runningMode: "IMAGE",
      outputCategoryMask: false,
      outputConfidenceMasks: true,
    });
  })();
  return segmenterPromise;
}

/// Segment the given RGBA buffer and return a foreground probability mask.
/// Returns null if segmentation fails.
export async function extractSegmentationMask(
  rgba: Uint8ClampedArray,
  width: number,
  height: number,
): Promise<Float32Array | null> {
  try {
    const segmenter = await loadSegmenter();
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    canvas.getContext("2d")!.putImageData(new ImageData(rgba, width, height), 0, 0);

    const result = segmenter.segment(canvas);
    const masks = result.confidenceMasks;
    if (!masks || masks.length === 0) return null;

    // selfie_segmenter_landscape outputs 2 classes:
    // [0] = background confidence, [1] = person confidence.
    // If only 1 mask is present, it is the foreground mask.
    const fgMask = masks.length > 1 ? masks[1] : masks[0];
    const data = fgMask.getAsFloat32Array();

    // Close masks to free WebGL resources.
    for (const m of masks) m.close();

    return data;
  } catch (e) {
    console.warn("segmentation failed:", e);
    return null;
  }
}
