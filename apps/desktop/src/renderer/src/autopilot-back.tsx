// When the owner is back: while any Autopilot runs, the first key or press anywhere in the window after ten
// minutes with neither asks whether to keep it on. It keeps running until they answer; nothing turns off by
// itself. Keeping it on closes the question, and it comes back only after ten more minutes with no input.
// Presses inside the switch or the setup are the owner using Autopilot, so they are never the one that asks.
// **The press that asks does not also act on what was under it.** Its own key-up, or its mouse-up and click, are
// eaten too.

import { useEffect, useState } from "react";
import { Dialog } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { stopRunning, useRunning } from "./cockpit/deck/autopilots";
import { WALK_UI } from "./walk-find";

const AWAY_MS = 10 * 60_000;
/** The key that turns it off from the question. */
const OFF_KEY = "t";
/** The owner's own controls for Autopilot: nothing pressed inside them is the first use of the window. */
const OWN = `[${WALK_UI}], [data-keeps-autopilot], [data-hud="autopilot"], .ap-hud`;

const watching: { on: boolean; asking: boolean; ask: () => void } = { on: false, asking: false, ask: () => undefined };
let lastInput = Date.now();

/** Time passing with nobody at the window, for a mock that cannot wait ten minutes. */
export const minutesPass = (ms: number): void => {
  lastInput -= ms;
};

/** Eats the rest of the gesture that asked, so it does not land on what was under it. */
function eatRest(types: readonly string[], last: string): void {
  const eat = (event: Event) => {
    event.preventDefault();
    event.stopImmediatePropagation();
    if (event.type === last) end();
  };
  const end = () => {
    types.forEach((type) => window.removeEventListener(type, eat, true));
    window.clearTimeout(timer);
  };
  const timer = window.setTimeout(end, 600);
  types.forEach((type) => window.addEventListener(type, eat, true));
}

function used(event: Event): void {
  if (!watching.on) return;
  const now = Date.now();
  const away = now - lastInput >= AWAY_MS;
  lastInput = now;
  if (!away || watching.asking) return;
  if (event.target instanceof Element && event.target.closest(OWN) !== null) return;
  watching.asking = true;
  event.preventDefault();
  event.stopImmediatePropagation();
  eatRest(event.type === "keydown" ? ["keypress", "keyup"] : ["mousedown", "pointerup", "mouseup", "click"], event.type === "keydown" ? "keyup" : "click");
  watching.ask();
}

// Registered when this module loads, ahead of the listeners the windows add as they mount, so the press that asks is seen before anything under it.
if (typeof window !== "undefined") {
  window.addEventListener("keydown", used, true);
  window.addEventListener("pointerdown", used, true);
}

export function AutopilotBack({ repositories }: { repositories: readonly RepositorySummary[] }) {
  const running = useRunning();
  const on = running.length > 0;
  const [asking, setAsking] = useState(false);

  useEffect(() => {
    watching.on = on;
    watching.ask = () => setAsking(true);
    if (on) lastInput = Date.now();
    else {
      watching.asking = false;
      setAsking(false);
    }
    return () => {
      watching.on = false;
    };
  }, [on]);

  const keep = () => {
    lastInput = Date.now();
    watching.asking = false;
    setAsking(false);
  };
  const off = () => {
    keep();
    void stopRunning();
  };

  useEffect(() => {
    if (!asking) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      // Enter and Escape are the layer's own: Keep it on is its Cancel, which holds focus and answers both.
      if (event.key.toLowerCase() === OFF_KEY) {
        event.preventDefault();
        event.stopImmediatePropagation();
        off();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [asking]);

  const nameOf = (manifest: string) => repositories.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest;
  return (
    <Dialog open={asking && on} title="Autopilot is running" tone="neutral" confirmLabel="Turn it off" cancelLabel="Keep it on" onConfirm={off} onCancel={keep}>
      <ul>
        {running.map((one) => (
          <li key={one.manifest_id}>{nameOf(one.manifest_id)}</li>
        ))}
      </ul>
    </Dialog>
  );
}
