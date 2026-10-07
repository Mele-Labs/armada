// Bridge's own tests, in four projects that two Checks run: `desktop` and
// `desktop (browser)` for what reads no surface, `smoke` and `smoke (browser)`
// for what does. None runs with Electron.
//
// **The split is by what a test imports, not by what it is about.** `apps/desktop/armada.yml`
// says which paths hit which Check. A test belongs in `smoke` when the modules
// it loads reach a surface's code at runtime: every test under `mock/` (the
// mock Fleet composes every surface through `fake.ts`, and `App` mounts them),
// `annotate/Layer.test.tsx` (`openJobIn` from Jobs) and `left-column.test.ts`
// (Overview's readings). The rest load `src/main`, `src/shared`, the
// renderer's own modules and the shared packages, and the surfaces only as
// types, which a bundler drops.
//
// **The fourth runner, and the first that reaches `src/main`.** The other three
// are `packages/screens`, twice — its modules in node and its screens in a
// browser — and `packages/components`' stories.
//
// **`node`, and nothing from Electron.** A test that needed `app` or a
// `BrowserWindow` would be a test of the shell rather than of the state. The
// renderer's node tests are for folds like `where-open.ts`', which need no window.
//
// **The browser projects run in Chromium.** `smoke (browser)` mounts `App` on
// the mock Fleet, `src/renderer/src/mock/`. `.test.tsx` is the browser here, as
// it is in `packages/screens`. Its settings are in `vitest.browser.preset.ts`,
// which surfaces share.
import { configDefaults, defineConfig } from "vitest/config";

import { browserProject } from "./vitest.browser.preset";
import WeightedShards from "./vitest.shard";

// What loads a surface's code, by path under `src/renderer/src`.
const smokeNode = ["src/renderer/src/mock/**/*.test.ts", "src/renderer/src/left-column.test.ts"];
const smokeBrowser = ["src/renderer/src/mock/**/*.test.tsx", "src/renderer/src/annotate/Layer.test.tsx"];

// The preset takes an `include` and nothing else, so what `smoke (browser)` owns is taken out here.
const desktopBrowser = browserProject("desktop (browser)", ["src/renderer/**/*.test.tsx"]);
desktopBrowser.test = { ...desktopBrowser.test, exclude: [...configDefaults.exclude, ...smokeBrowser] };

export default defineConfig({
  test: {
    sequence: { sequencer: WeightedShards },
    projects: [
      {
        test: {
          name: "desktop",
          environment: "node",
          include: ["src/main/**/*.test.ts", "src/shared/**/*.test.ts", "src/renderer/**/*.test.ts"],
          exclude: smokeNode,
        },
      },
      desktopBrowser,
      {
        test: {
          name: "smoke",
          environment: "node",
          include: smokeNode,
        },
      },
      browserProject("smoke (browser)", smokeBrowser),
    ],
  },
});
