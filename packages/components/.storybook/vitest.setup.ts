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
