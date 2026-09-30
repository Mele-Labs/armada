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
// `.test.tsx` is the browser here, as it is in `packages/screens`.
import tailwindcss from "@tailwindcss/vite";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";
import type { BrowserCommand } from "vitest/node";

/** Motion on or off for the page a test runs in, where every test starts with it reduced. */
const motion: BrowserCommand<[on: boolean]> = ({ page }, on) =>
  page.emulateMedia({ reducedMotion: on ? "no-preference" : "reduce" });

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
      {
        // The app's stylesheet imports Tailwind, so the plugin that compiles it is here too.
        plugins: [tailwindcss()],
        // Found only once a test renders JSX, and Vite reloads the test when it finds it.
        optimizeDeps: { include: ["react/jsx-dev-runtime"] },
        test: {
          name: "renderer (browser)",
          include: ["src/renderer/**/*.test.tsx"],
          // Every guide already met, so a card opening on a piece nobody has
          // seen does not sit over the press a test about something else is
          // making. `mock/guides-met.ts` carries the reasoning.
          setupFiles: ["./src/renderer/src/mock/guides-met.ts"],
          // **Five seconds for a poll, not vitest's one.** Every
          // `expect.poll` and `expect.element` here waits for a render, and
          // one second is the budget on a quiet machine — the merge line runs
          // this beside 3,835 Rust tests, and `board-gone` and `canvas-pan`
          // each refused it on 30 Sep 2026 for a render that had not landed
          // yet. Raising the ceiling costs nothing a passing test spends: a
          // poll returns the moment it succeeds, and the slowest passing test
          // in the five heaviest files measures 1,573ms. What it does cost is
          // a genuinely failing poll, which now waits five seconds instead of
          // one — rare enough to be worth the trade against a red merge line.
          expect: { poll: { timeout: 5_000 } },
          browser: {
            enabled: true,
            headless: true,
            // **Three seconds, not Playwright's fifteen.** A file is one
            // worker and its tests run in order, so a file with 38 failing
            // tests spent 38 × 15s waiting for elements that were never
            // going to appear — 570s in one file, which no number of cores
            // shortens. Measured before choosing it: the slowest passing
            // test in the five heaviest files is 1,573ms, and the whole mock
            // suite went from 577s to 18s with no test changing colour.
            //
            // **Reduced motion, emulated.** Every `--duration-*` a panel moves
            // on is 0ms under it (`packages/tokens/src/motion.css`), so a sheet
            // or dialog is in place on its first frame and a press cannot land
            // on one still sliding in — `arc/plan-revision-refused` and a
            // Workflow test each did, under load, on 30 Sep 2026. A test whose
            // claim is the motion itself turns it back on with `motion()` in
            // `src/renderer/src/mock/testing.ts`.
            provider: playwright({ actionTimeout: 3_000, contextOptions: { reducedMotion: "reduce" } }),
            commands: { motion },
            instances: [{ browser: "chromium" }],
            viewport: { width: 1440, height: 900 },
          },
        },
      },
    ],
  },
});
