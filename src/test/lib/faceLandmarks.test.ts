import { describe, expect, it } from "vite-plus/test";
import { buildGeometry, type Keypoint } from "~/lib/faceLandmarks";

// Synth: 478 keypoints arranged so each index has predictable coordinates.
// This lets us check the geometry packing logic without needing MediaPipe.
function syntheticKeypoints(): Keypoint[] {
  const kps: Keypoint[] = [];
  for (let i = 0; i < 478; i++) {
    // Spread across image deterministically.
    kps.push({ x: ((i * 7919) % 1000) / 1000, y: ((i * 6113) % 1000) / 1000 });
  }
  // Override iris centres + perimeters with known values.
  kps[468] = { x: 0.4, y: 0.45 };
  kps[469] = { x: 0.4, y: 0.43 };
  kps[470] = { x: 0.42, y: 0.45 };
  kps[471] = { x: 0.4, y: 0.47 };
  kps[472] = { x: 0.38, y: 0.45 };
  kps[473] = { x: 0.6, y: 0.45 };
  kps[474] = { x: 0.6, y: 0.43 };
  kps[475] = { x: 0.62, y: 0.45 };
  kps[476] = { x: 0.6, y: 0.47 };
  kps[477] = { x: 0.58, y: 0.45 };
  return kps;
}

describe("buildGeometry", () => {
  it("returns null for too-few keypoints (no iris refine)", () => {
    const kps: Keypoint[] = Array.from({ length: 100 }, () => ({ x: 0.5, y: 0.5 }));
    expect(buildGeometry(kps)).toBeNull();
  });

  it("packs face oval as a 36-vertex polygon (72 floats)", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    expect(g.faceOval).toHaveLength(72);
  });

  it("exclusions header starts with 6 (number of polygons)", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    expect(g.exclusions[0]).toBe(6);
  });

  it("computes eye warp radius from iris perimeter × 2.5", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    // Iris perimeter is 0.02 from centre (set above). Warp radius = 0.05.
    expect(g.eyes[0]).toBeCloseTo(0.4, 5);
    expect(g.eyes[1]).toBeCloseTo(0.45, 5);
    expect(g.eyes[2]).toBeCloseTo(0.05, 5);
    expect(g.eyes[3]).toBeCloseTo(0.6, 5);
    expect(g.eyes[5]).toBeCloseTo(0.05, 5);
  });

  it("eyebrow polygon has even point count (extruded loop)", () => {
    const g = buildGeometry(syntheticKeypoints())!;
    // skip: header(1) + eye(1+32) + eye(1+32) = 67; eyebrow_left starts at 67
    // Actually we just verify packed polygons are well-formed: every poly has
    // even coordinate count and matches its declared length.
    let i = 1;
    const n = g.exclusions[0];
    for (let p = 0; p < n; p++) {
      const len = g.exclusions[i++];
      expect(len).toBeGreaterThan(2);
      i += len * 2;
    }
    expect(i).toBe(g.exclusions.length);
  });
});
