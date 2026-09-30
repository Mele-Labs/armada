// Motion back on, for a story whose claim is the motion itself.
//
// **The test run emulates reduced motion** (`vitest.config.ts`), so every panel is in place on its
// first frame. A story that proves a hold filling, a sweep running or a loop looping says
// `parameters: { motion: "on" }`, and `preview.tsx`'s `beforeEach` brings motion back for it
// alone. The switch is a Playwright call, so only the test run can make it: `vitest.setup.ts`
// hands it here, and in `storybook dev` there is none and a story sees the machine's preference.

type Emulate = (on: boolean) => Promise<void>;

let emulate: Emulate | null = null;

/** The test run's switch. `vitest.setup.ts` calls this, and nothing else does. */
export function emulateMotionWith(switchTo: Emulate): void {
  emulate = switchTo;
}

/** Whether this is the test run, which starts every story with reduced motion emulated. */
export function motionEmulated(): boolean {
  return emulate !== null;
}

/** Motion on for a story that asks for it, and the cleanup that puts reduced motion back. */
export async function motionFor(parameters: { motion?: "on" }): Promise<(() => Promise<void>) | undefined> {
  const switchTo = emulate;
  if (parameters.motion !== "on" || switchTo === null) return undefined;
  await switchTo(true);
  return () => switchTo(false);
}
