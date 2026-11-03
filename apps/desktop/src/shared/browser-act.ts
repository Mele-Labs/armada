// What the Job browser says an agent act is doing, one sentence of facts. Main and the renderer both read it.

import type { BrowserAct } from "@armada/protocol";

/** The host of an address, else the address as given. */
const hostOf = (url: string): string => {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
};

/** The words a target is known by: its name, else its role. */
const named = (target: { role: string; name?: string }): string => target.name ?? target.role;

/** The sentence for an act that is running. */
export function describeAct(act: BrowserAct): string {
  switch (act.kind) {
    case "open":
      return `Opening ${hostOf(act.url)}`;
    case "click":
      return `Clicking ${named(act.target)}`;
    case "type":
      return `Typing in ${named(act.target)}`;
    case "snapshot":
      return "Reading the page";
    case "screenshot":
      return "Taking a picture of the page";
    case "wait_for": {
      const what = act.text ?? (act.target === undefined ? undefined : named(act.target));
      return what === undefined ? "Waiting for the page" : `Waiting for ${what}`;
    }
    case "evaluate":
      return "Running a script";
  }
}
