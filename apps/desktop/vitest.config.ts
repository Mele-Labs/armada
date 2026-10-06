// Main's own state machines and the renderer's own, in node, and the renderer
// whole in Chromium — none with Electron.
//
// **The fourth runner, and the first that reaches `src/main`.** The other three
// are `packages/screens`, twice — its modules in node and its screens in a
// browser — and `packages/components`' stories.
//
// **`node`, and nothing from Electron.** A test that needed `app` or a
// `BrowserWindow` would be a test of the shell rather than of the state. The
// renderer's node project is for folds like `where-open.ts`', which need no window.
//
// **The browser project mounts `App` on the mock Fleet**, `src/renderer/src/mock/`.
// `.test.tsx` is the browser here, as it is in `packages/screens`. Its settings
// are in `vitest.browser.preset.ts`, which surfaces share.
import { defineConfig } from "vitest/config";

import { browserProject } from "./vitest.browser.preset";

export default defineConfig({
  test: {
    projects: [
      {
        test: {
          name: "main",
          environment: "node",
          include: ["src/main/**/*.test.ts"],
        },
      },
      {
        test: {
          name: "renderer",
          environment: "node",
          include: ["src/renderer/**/*.test.ts"],
        },
      },
      browserProject("renderer (browser)", ["src/renderer/**/*.test.tsx"]),
    ],
  },
});
