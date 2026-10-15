// The test run's half of `./motion.ts`: the Playwright call that turns motion back on, which a
// story in `storybook dev` has no way to make.

import { commands } from "vitest/browser";

import { emulateMotionWith } from "./motion";

declare module "vitest/browser" {
  interface BrowserCommands {
    /** `vitest.config.ts`' own: motion on, or back to reduced. */
    motion: (on: boolean) => Promise<void>;
  }
}

emulateMotionWith((on) => commands.motion(on));

// Storybook runs a story's `beforeEach` cleanup when the *next* story starts, so a file's last
// motion-on story never hands motion back. Files run one after another in a page, and the
// emulation is the page's, so the next file would start with motion on. Reduce it per file.
await commands.motion(false);
