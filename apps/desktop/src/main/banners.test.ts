// Why a shown banner is held: Electron collects a `Notification` nobody references, and a collected one
// never fires "click", so pressing it only activates the app. It is let go once it can no longer be pressed.

import { describe, expect, it } from "vitest";

import { isHeld, showHeld } from "./banners";
import type { Banner } from "./banners";

type Event = "click" | "close" | "failed";

function aBanner(): { banner: Banner; fire: (event: Event) => void; shown: () => number } {
  const listeners = new Map<Event, () => void>();
  let shown = 0;
  const banner: Banner = {
    on: (event, listener) => {
      listeners.set(event, listener);
    },
    show: () => {
      shown += 1;
    },
  };
  return { banner, fire: (event) => listeners.get(event)?.(), shown: () => shown };
}

describe("a shown banner", () => {
  it("is shown", () => {
    const { banner, shown } = aBanner();
    showHeld(banner);
    expect(shown()).toBe(1);
  });

  it("is held, since nothing else keeps it alive", () => {
    const { banner } = aBanner();
    showHeld(banner);
    expect(isHeld(banner)).toBe(true);
  });

  it.each(["click", "close", "failed"] as const)("is let go after %s", (event) => {
    const { banner, fire } = aBanner();
    showHeld(banner);
    fire(event);
    expect(isHeld(banner)).toBe(false);
  });
});
