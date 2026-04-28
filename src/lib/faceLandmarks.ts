// MediaPipe FaceMesh wrapper.
//
// Returns the geometry the WASM pipeline needs: face oval polygon, packed
// exclusion polygons (eyes, eyebrows, lips, nostrils), and per-eye iris
// centres + warp radii.
//
// The detector and TFJS backend are lazy-loaded so they don't bloat the
// initial bundle — they're only fetched the first time `extractGeometry` is
// called.

import type { FaceLandmarksDetector } from "@tensorflow-models/face-landmarks-detection";

// MediaPipe FaceMesh keypoint indices.
// Reference: https://github.com/google-ai-edge/mediapipe/blob/master/mediapipe/python/solutions/face_mesh_connections.py
//
// 36 vertices that trace the face oval clockwise.
const FACE_OVAL = [
  10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288, 397, 365, 379, 378, 400, 377, 152, 148,
  176, 149, 150, 136, 172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109,
];

// Closed eye contours (subject's perspective).
const LEFT_EYE = [263, 466, 388, 387, 386, 385, 384, 398, 362, 382, 381, 380, 374, 373, 390, 249];
const RIGHT_EYE = [33, 246, 161, 160, 159, 158, 157, 173, 133, 155, 154, 153, 145, 144, 163, 7];

// Upper edge of each eyebrow. Bottom edge synthesised by offsetting Y down.
const LEFT_EYEBROW = [336, 296, 334, 293, 300, 285, 295, 282, 283, 276];
const RIGHT_EYEBROW = [70, 63, 105, 66, 107, 55, 65, 52, 53, 46];

// Outer lip contour.
const LIPS_OUTER = [
  61, 185, 40, 39, 37, 0, 267, 269, 270, 409, 291, 375, 321, 405, 314, 17, 84, 181, 91, 146,
];

// Iris keypoints (only present with refineLandmarks: true).
// First index is centre, next four are perimeter.
const LEFT_IRIS_CENTER = 468;
const RIGHT_IRIS_CENTER = 473;

// Nose tip area for the nostril mask.
const NOSE_TIP = 2;

const EYE_WARP_RADIUS_FACTOR = 2.5;
const EYEBROW_THICKNESS = 0.025; // normalized — extruded downward from upper line
const NOSTRIL_HALF_W = 0.04;
const NOSTRIL_OFFSET_TOP = -0.005;
const NOSTRIL_OFFSET_BOTTOM = 0.025;

let detectorPromise: Promise<FaceLandmarksDetector> | null = null;

export async function loadDetector(): Promise<FaceLandmarksDetector> {
  if (detectorPromise) return detectorPromise;
  detectorPromise = (async () => {
    // Backend MUST be registered before the model is created.
    await import("@tensorflow/tfjs-backend-webgl");
    const fld = await import("@tensorflow-models/face-landmarks-detection");
    return fld.createDetector(fld.SupportedModels.MediaPipeFaceMesh, {
      runtime: "tfjs",
      refineLandmarks: true,
      maxFaces: 1,
    });
  })();
  return detectorPromise;
}

export interface Keypoint {
  x: number;
  y: number;
}

export interface FaceGeometry {
  faceOval: Float32Array;
  exclusions: Float32Array;
  eyes: Float32Array;
}

/// Pure geometry extraction — separated from detector I/O for testability.
/// `keypoints[i]` must already be normalised to 0..1 (x by width, y by height).
export function buildGeometry(keypoints: Keypoint[]): FaceGeometry | null {
  if (keypoints.length < 478) return null; // refineLandmarks did not run

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

  // Eyebrows: extrude downward to give the upper-line indices thickness.
  const extrudeDown = (top: number[], dy: number): number[] => {
    const result = [...top];
    for (let i = top.length - 2; i >= 0; i -= 2) {
      result.push(top[i], top[i + 1] + dy);
    }
    return result;
  };
  const leftEyebrow = extrudeDown(collect(LEFT_EYEBROW), EYEBROW_THICKNESS);
  const rightEyebrow = extrudeDown(collect(RIGHT_EYEBROW), EYEBROW_THICKNESS);

  // Nostril rectangle around nose tip.
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

  // Iris geometry. Centre at index 0 of the iris cluster, perimeter at 1..4.
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

  return {
    faceOval: new Float32Array(faceOval),
    exclusions: new Float32Array(exclusions),
    eyes: new Float32Array(eyes),
  };
}

/// Detect a face in the given RGBA buffer and return geometry, or null if no
/// face was found.
export async function extractGeometry(
  rgba: Uint8ClampedArray,
  width: number,
  height: number,
): Promise<FaceGeometry | null> {
  const detector = await loadDetector();
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  canvas.getContext("2d")!.putImageData(new ImageData(rgba, width, height), 0, 0);

  const faces = await detector.estimateFaces(canvas);
  if (!faces.length) return null;
  const kpsRaw = faces[0].keypoints;
  // Normalise to 0..1.
  const kps: Keypoint[] = kpsRaw.map((p) => ({ x: p.x / width, y: p.y / height }));
  return buildGeometry(kps);
}
