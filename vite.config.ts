import { defineConfig } from "vite-plus";
import preact from "@preact/preset-vite";
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";
import { resolve, extname } from "node:path";
import { createReadStream, existsSync } from "node:fs";

const MEDIAPIPE_WASM_DIR = resolve(
  import.meta.dirname,
  "node_modules/@mediapipe/tasks-vision/wasm",
);

const mediapipeLocalPlugin = {
  name: "mediapipe-local-wasm",
  configureServer(server: {
    middlewares: {
      use: (path: string, fn: (req: any, res: any, next: () => void) => void) => void;
    };
  }) {
    server.middlewares.use("/mediapipe-wasm", (req: any, res: any, next: () => void) => {
      const filePath = resolve(MEDIAPIPE_WASM_DIR, req.url.replace(/^\//, "").split("?")[0]);
      if (!filePath.startsWith(MEDIAPIPE_WASM_DIR) || !existsSync(filePath)) {
        next();
        return;
      }
      const ext = extname(filePath);
      const mime = ext === ".wasm" ? "application/wasm" : "application/javascript";
      res.setHeader("Content-Type", mime);
      res.setHeader("Cross-Origin-Resource-Policy", "cross-origin");
      createReadStream(filePath).pipe(res);
    });
  },
};

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  fmt: {},
  plugins: [wasm(), tailwindcss(), preact(), mediapipeLocalPlugin],
  server: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  resolve: {
    alias: {
      "~": resolve(import.meta.dirname, "src"),
    },
  },
  optimizeDeps: {
    exclude: ["purikura-wasm"],
  },
  build: {
    target: "esnext",
  },
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: ["src/test/setup.ts"],
    coverage: {
      provider: "v8",
      include: ["src/lib/**", "src/state/**", "src/hooks/**"],
      thresholds: {
        lines: 90,
        statements: 90,
        functions: 90,
        branches: 85,
      },
    },
  },
});
