// The annotation layer, on, over notes the scenario holds, for a walk to play. **A scenario named
// here has no real sink behind it**, so the layer is given one that keeps the notes in memory and
// answers a capture with a picture, so that sending is offered as it is in Bridge itself.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import type { Annotation } from "../../../shared/annotations";
import { Layer, ON_KEY } from "../annotate/Layer";
import type { Sink } from "../annotate/sink";

const AT = "2026-10-07T13:48:02.000Z";

const note = (id: string, text: string, selector: string, tag: string, label: string, source: string, box: Annotation["box"]): Annotation => ({
  id,
  status: "open",
  text,
  component: label,
  source: { file: source, line: 88 },
  owners: [],
  ownersFrom: "parent",
  selector,
  element: { tag, text: "", label: null },
  screen: "Job Board",
  layer: null,
  location: "/",
  scenario: null,
  box,
  window: { width: 1440, height: 900 },
  createdAt: AT,
  updatedAt: AT,
});

const NOTES: Record<string, () => Annotation[]> = {
  "annotate-to-session": () => [
    note("20261007-134802-aaaa", "The title is cut off in the row", ".armada-dtabs", "div", "DashboardTabs", "packages/surfaces/overview/src/DashboardTabs.tsx", { x: 300, y: 120, width: 900, height: 200 }),
    note("20261007-134905-bbbb", "The rail has no room for a third group", 'nav[aria-label="Work"]', "nav", "Rail", "packages/shell/src/Rail.tsx", { x: 0, y: 0, width: 220, height: 800 }),
  ],
};

/** A one-pixel PNG, which is all a capture has to be here. */
const PNG = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="), (c) => c.charCodeAt(0)).buffer;

function sinkOf(notes: Annotation[]): Sink {
  const held = new Map(notes.map((one) => [one.id, one]));
  return {
    via: "main",
    list: async () => [...held.values()],
    save: async (one) => {
      held.set(one.id, one);
    },
    remove: async (id) => {
      held.delete(id);
    },
    root: async () => "/Users/user/code/armada",
    capture: async () => PNG,
  };
}

/** Whether the mock page draws this scenario's own layer, and so leaves the dev server's off. */
export const annotatesWith = (scenario: string): boolean => scenario in NOTES;

/** The layer over `scenario`'s notes, or nothing where it has none. Returns how to take it down. */
export function mountAnnotating(scenario: string): () => void {
  const notes = NOTES[scenario];
  if (notes === undefined) return () => undefined;
  try {
    sessionStorage.setItem(ON_KEY, "1");
  } catch {
    // The layer then opens off, and a walk looking for its bar says so.
  }
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Layer sink={sinkOf(notes())} />
    </StrictMode>,
  );
  return () => {
    try {
      sessionStorage.removeItem(ON_KEY);
    } catch {
      // Nothing was kept.
    }
    root.unmount();
    host.remove();
  };
}
