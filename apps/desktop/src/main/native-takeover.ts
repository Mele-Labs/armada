// While the native Armada app runs it tells the owner a Job needs him, and Armada Classic stays quiet.
//
// **Two apps telling him the same thing is twice the noise.** The native app owns the banner, its sound and
// the Dock count (`crates/bridge-native/src/notify.rs`). This is how Classic knows to stand down: it looks for
// the native app's bundle id among the running apps, and takes its badge back the moment that app quits.
// Nothing else in Classic changes. The effects arrive as functions, so this runs in node with no Electron in it.

import { execFile } from "node:child_process";

/** The native app's `CFBundleIdentifier`, as `scripts/native-app` writes it. Not Classic's own, `dev.armada.bridge`. */
export const NATIVE_BUNDLE_ID = "dev.armada.bridge.native";

/** How often the running apps are looked at: slow enough to cost nothing, quick enough that two apps are never loud for long. */
export const WATCH_MS = 3000;

/** The Dock badge text for a count: empty at 0. */
const labelOf = (count: number): string => (count === 0 ? "" : String(count));

/**
 * Whether `lsappinfo find bundleid=<id>` found a running app. It prints an `ASN:` line for one and nothing for none,
 * and exits 0 either way. Looking it up never launches the app.
 */
export function foundRunning(output: string): boolean {
  return /^ASN:/m.test(output);
}

export type Effects = {
  /** Write Classic's Dock badge. */
  badge: (label: string) => void;
};

export class NativeTakeover {
  private readonly effects: Effects;
  private running = false;
  /** The count the window last sent, so the badge comes back right when the native app quits. */
  private count = 0;

  constructor(effects: Effects) {
    this.effects = effects;
  }

  /** True while the native app runs: no banner, no sound and no badge from Classic. */
  get active(): boolean {
    return this.running;
  }

  /** The Cockpit's count of calls. Badged unless the native app is the one badging. */
  dockCount(count: number): void {
    this.count = count;
    if (!this.running) this.effects.badge(labelOf(count));
  }

  /** One look at the running apps. The badge clears when the native app appears and returns when it is gone. */
  read(running: boolean): void {
    if (running === this.running) return;
    this.running = running;
    this.effects.badge(running ? "" : labelOf(this.count));
  }
}

/** Ask the system whether the native app is running. `null` where the question itself failed: that is no answer, and the last reading stands. */
export function lookForNative(): Promise<boolean | null> {
  return new Promise((resolve) => {
    execFile(
      "/usr/bin/lsappinfo",
      [
        "find",
        `bundleid=${NATIVE_BUNDLE_ID}`,
      ],
      {
        timeout: 2000,
      },
      (error, out) => resolve(error === null ? foundRunning(out) : null),
    );
  });
}

/** Look now and then every `WATCH_MS`. The timer does not keep the app alive. */
export function watchNative(
  takeover: NativeTakeover,
  look: () => Promise<boolean | null> = lookForNative,
  every: number = WATCH_MS,
): () => void {
  const once = (): void => {
    void look().then((found) => {
      if (found !== null) takeover.read(found);
    });
  };
  once();
  const timer = setInterval(once, every);
  timer.unref();
  return () => clearInterval(timer);
}
