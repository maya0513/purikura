// MediaPipe FaceLandmarker wrapper (@mediapipe/tasks-vision).
//
// Returns the geometry the WASM pipeline needs: face oval polygon, packed
// exclusion polygons (eyes, eyebrows, lips, nostrils), per-eye iris
// centres + warp radii, and makeup geometry (lip polygon, eye polygons, cheeks).
//
// The landmarker is lazy-loaded from the MediaPipe CDN; subsequent calls
// are served from the cached instance.

import { FaceLandmarker, FilesetResolver } from "@mediapipe/tasks-vision";

// MediaPipe FaceMesh keypoint indices.
// 36 vertices tracing the face oval clockwise.
const FACE_OVAL = [
  10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288, 397, 365, 379, 378, 400, 377, 152, 148,
  176, 149, 150, 136, 172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109,
];

const LEFT_EYE = [263, 466, 388, 387, 386, 385, 384, 398, 362, 382, 381, 380, 374, 373, 390, 249];
const RIGHT_EYE = [33, 246, 161, 160, 159, 158, 157, 173, 133, 155, 154, 153, 145, 144, 163, 7];

const LEFT_EYEBROW = [336, 296, 334, 293, 300, 285, 295, 282, 283, 276];
const RIGHT_EYEBROW = [70, 63, 105, 66, 107, 55, 65, 52, 53, 46];

const LIPS_OUTER = [
  61, 185, 40, 39, 37, 0, 267, 269, 270, 409, 291, 375, 321, 405, 314, 17, 84, 181, 91, 146,
];

// Iris keypoints (available with FaceLandmarker, which always uses refineLandmarks).
const LEFT_IRIS_CENTER = 468;
const RIGHT_IRIS_CENTER = 473;
const NOSE_TIP = 2;

// Cheekbone landmarks (widest face point on each side).
const LEFT_CHEEK_IDX = 234;
const RIGHT_CHEEK_IDX = 454;

const EYE_WARP_RADIUS_FACTOR = 2.5;
const EYEBROW_THICKNESS = 0.025;
const NOSTRIL_HALF_W = 0.04;
const NOSTRIL_OFFSET_TOP = -0.005;
const NOSTRIL_OFFSET_BOTTOM = 0.025;

// In dev mode serve WASM locally (avoids Firefox COEP/CORS issues with CDN).
const MP_BASE = import.meta.env.DEV
  ? "/mediapipe-wasm"
  : "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.22/wasm";
const MODEL_URL =
  "https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task";

let landmarkerPromise: Promise<FaceLandmarker> | null = null;

export async function loadDetector(): Promise<FaceLandmarker> {
  if (landmarkerPromise) return landmarkerPromise;
  landmarkerPromise = (async () => {
    const vision = await FilesetResolver.forVisionTasks(MP_BASE);
    return FaceLandmarker.createFromOptions(vision, {
      baseOptions: { modelAssetPath: MODEL_URL, delegate: "GPU" },
      runningMode: "IMAGE",
      numFaces: 1,
      minFaceDetectionConfidence: 0.5,
      minFacePresenceScore: 0.5,
      minTrackingConfidence: 0.5,
    });
  })();
  return landmarkerPromise;
}

export interface Keypoint {
  x: number;
  y: number;
}

export interface FaceGeometry {
  faceOval: Float32Array;
  exclusions: Float32Array;
  eyes: Float32Array;
  // Makeup geometry
  lipsOuter: Float32Array;
  leftEye: Float32Array;
  rightEye: Float32Array;
  cheeks: Float32Array; // [cx_left, cy_left, cx_right, cy_right]
}

/// Pure geometry extraction — separated from I/O for testability.
/// `keypoints[i]` must already be normalised to 0..1.
export function buildGeometry(keypoints: Keypoint[]): FaceGeometry | null {
  if (keypoints.length < 478) return null;

  const collect = (indices: number[]): number[] => {
    const out: number[] = [];
    for (const i of indices) {
      out.push(keypoints[i].x, keypoints[i].y);
    }
    return out;
  };

  const faceOval = collect(FACE_OVAL);
  const leftEye = collect(LEFT_EYE);
  const rightEye = collect(RIGHT_EYE);
  const lips = collect(LIPS_OUTER);

  const extrudeDown = (top: number[], dy: number): number[] => {
    const result = [...top];
    for (let i = top.length - 2; i >= 0; i -= 2) {
      result.push(top[i], top[i + 1] + dy);
    }
    return result;
  };
  const leftEyebrow = extrudeDown(collect(LEFT_EYEBROW), EYEBROW_THICKNESS);
  const rightEyebrow = extrudeDown(collect(RIGHT_EYEBROW), EYEBROW_THICKNESS);

  const noseTip = keypoints[NOSE_TIP];
  const nostrils = [
    noseTip.x - NOSTRIL_HALF_W,
    noseTip.y + NOSTRIL_OFFSET_TOP,
    noseTip.x + NOSTRIL_HALF_W,
    noseTip.y + NOSTRIL_OFFSET_TOP,
    noseTip.x + NOSTRIL_HALF_W,
    noseTip.y + NOSTRIL_OFFSET_BOTTOM,
    noseTip.x - NOSTRIL_HALF_W,
    noseTip.y + NOSTRIL_OFFSET_BOTTOM,
  ];

  const polys = [leftEye, rightEye, leftEyebrow, rightEyebrow, lips, nostrils];
  const exclusions: number[] = [polys.length];
  for (const p of polys) {
    exclusions.push(p.length / 2, ...p);
  }

  const irisRadius = (centerIdx: number): number => {
    const c = keypoints[centerIdx];
    let r = 0;
    for (let k = 1; k <= 4; k++) {
      const p = keypoints[centerIdx + k];
      const dx = p.x - c.x;
      const dy = p.y - c.y;
      r = Math.max(r, Math.sqrt(dx * dx + dy * dy));
    }
    return r;
  };
  const lc = keypoints[LEFT_IRIS_CENTER];
  const rc = keypoints[RIGHT_IRIS_CENTER];
  const eyes = [
    lc.x,
    lc.y,
    irisRadius(LEFT_IRIS_CENTER) * EYE_WARP_RADIUS_FACTOR,
    rc.x,
    rc.y,
    irisRadius(RIGHT_IRIS_CENTER) * EYE_WARP_RADIUS_FACTOR,
  ];

  const lcheek = keypoints[LEFT_CHEEK_IDX];
  const rcheek = keypoints[RIGHT_CHEEK_IDX];

  return {
    faceOval: new Float32Array(faceOval),
    exclusions: new Float32Array(exclusions),
    eyes: new Float32Array(eyes),
    lipsOuter: new Float32Array(lips),
    leftEye: new Float32Array(leftEye),
    rightEye: new Float32Array(rightEye),
    cheeks: new Float32Array([lcheek.x, lcheek.y, rcheek.x, rcheek.y]),
  };
}

/// Detect a face in the given RGBA buffer and return geometry, or null.
export async function extractGeometry(
  rgba: Uint8ClampedArray,
  width: number,
  height: number,
): Promise<FaceGeometry | null> {
  const landmarker = await loadDetector();
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  canvas.getContext("2d")!.putImageData(new ImageData(rgba, width, height), 0, 0);

  const result = landmarker.detect(canvas);
  if (!result.faceLandmarks.length) return null;

  // tasks-vision landmarks are already normalized to 0..1.
  const kps: Keypoint[] = result.faceLandmarks[0].map((p) => ({ x: p.x, y: p.y }));
  return buildGeometry(kps);
}
