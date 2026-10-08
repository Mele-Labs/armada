// The browser settings every Bridge test project shares: the app's own
// `renderer (browser)` project in `vitest.config.ts` and, in time, each
// `packages/surfaces/<x>` package. Reached as `@armada/desktop/vitest-preset`.
//
// **A surface's `vitest.config.ts` calls `browserProject` with its own name and
// `include`, and nothing else.** Viewport, reduced motion, the poll and action
// timeouts and the guides-met setup are decided here once, with the
// measurements behind them. A surface that wants a different number says why
// beside it, rather than copying this file and drifting.
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { playwright } from "@vitest/browser-playwright";
import type { TestProjectInlineConfiguration } from "vitest/config";
import type { BrowserCommand } from "vitest/node";

/** Motion on or off for the page a test runs in, where every test starts with it reduced. */
const motion: BrowserCommand<[on: boolean]> = ({ page }, on) =>
  page.emulateMedia({ reducedMotion: on ? "no-preference" : "reduce" });

/** Absolute, because a surface's project root is not this directory. */
const guidesMet = fileURLToPath(new URL("./src/renderer/src/mock/guides-met.ts", import.meta.url));

/** One browser project: Chromium on the mock Fleet, with every setting below. */
export function browserProject(name: string, include: string[]): TestProjectInlineConfiguration {
  return {
    // The app's stylesheet imports Tailwind, so the plugin that compiles it is here too.
    plugins: [tailwindcss()],
    // Found only once a test renders JSX, and Vite reloads the test when it finds it.
    optimizeDeps: { include: ["react/jsx-dev-runtime", "react-dom/server"] },
    test: {
      name,
      include,
      // Every guide already met, so a card opening on a piece nobody has
      // seen does not sit over the press a test about something else is
      // making. `mock/guides-met.ts` carries the reasoning.
      setupFiles: [guidesMet],
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
        // **Playwright's own fifteen seconds, restored 1 Oct 2026.**
        //
        // It was three for a day. The reason was good and it expired: on
        // 30 Sep this suite had 93 failures, a file is one worker and its
        // tests run in order, so `job-detail-width`'s 38 failures cost
        // 38 x 15s inside one file — 570s that no number of cores
        // shortens. Three seconds took the whole suite from 577s to 18s,
        // and the slowest passing test in the five heaviest files
        // measured 1,573ms, so the headroom was real when it was taken.
        //
        // **A day later the suite is green and twice the weight.** 571
        // tests where there were 516, with the plan gate's canvas and the
        // proposing fixtures in it, and the slowest passing test measures
        // **8,240ms**. Three seconds stopped being headroom and started
        // being the thing that fails: five `locator.click` calls timed out
        // on the merge line waiting for elements that existed, on a branch
        // that had not touched any of them.
        //
        // **A green suite spends nothing on this ceiling**, because a
        // passing action returns the moment it is actionable — the 36s
        // measured above is the same at three seconds and at fifteen. What
        // the ceiling buys is only ever paid by a failure, and the way to
        // stop paying it is to have none.
        //
        // So: no deviation without a live reason. If failures come back in
        // numbers, lower it again and write the measurement down — and
        // re-measure the slowest passing test first, which is the step
        // this comment exists because I skipped.
        provider: playwright({ contextOptions: { reducedMotion: "reduce" } }),
        commands: { motion },
        instances: [{ browser: "chromium" }],
        viewport: { width: 1440, height: 900 },
      },
    },
  };
}
