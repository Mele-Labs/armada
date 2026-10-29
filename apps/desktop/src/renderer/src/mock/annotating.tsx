// The annotation layer, on, over notes the scenario holds, for a walk to play. **A scenario named
// here has no real sink behind it**, so the layer is given one that keeps the notes in memory and
// answers a capture with a picture, so that sending is offered as it is in Bridge itself.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import type { Annotation } from "../../../shared/annotations";
import { Layer, ON_KEY } from "../annotate/Layer";
import { WiredSessions } from "../sessions-wired";
import type { Sink } from "../annotate/sink";

const AT = "2026-10-07T13:48:02.000Z";

const note = (id: string, text: string, selector: string, tag: string, label: string, source: string, box: Annotation["box"], screen: string | null = "Cockpit", extra: Partial<Annotation> = {}): Annotation => ({
  id,
  status: "open",
  text,
  component: label,
  source: { file: source, line: 88 },
  owners: [],
  ownersFrom: "parent",
  selector,
  element: { tag, text: "", label: null },
  screen,
  layer: null,
  location: "/",
  scenario: null,
  box,
  window: { width: 1440, height: 900 },
  createdAt: AT,
  updatedAt: AT,
  ...extra,
});

const NOTES: Record<string, () => Annotation[]> = {
  // The layer is mounted and off, as Bridge under `pnpm dev` runs it: ⇧⌘D is Debug Bridge.
  "debug-bridge-draft": () => [],
  // Off too: Report from a right press is what turns it on.
  "report-from-right-click": () => [],
  // One note on the Jobs list, for Start job to open a held Job. The layer is off until the walk turns it on, so the first two starts are not noted.
  "job-start-card-everywhere": () => [
    note("20261007-134802-aaaa", "The title is cut off in the row", 'section[aria-label="Jobs"]', "section", "Cockpit", "apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx", { x: 300, y: 120, width: 900, height: 200 }),
  ],
  "annotate-to-session": () => [
    note("20261007-134802-aaaa", "The title is cut off in the row", 'section[aria-label="Jobs"]', "section", "Cockpit", "apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx", { x: 300, y: 120, width: 900, height: 200 }),
    note("20261007-134905-bbbb", "The tabs have no room for a fifth place", 'nav[aria-label="Places"]', "nav", "TabBar", "apps/desktop/src/renderer/src/cockpit/deck/TabBar.tsx", { x: 0, y: 0, width: 220, height: 800 }),
    // The later ones are on other screens, whose elements the Cockpit does not draw.
    note("20261007-135100-cccc", "The ledger header wraps under the title", "#session-ledger-head", "header", "LedgerHead", "packages/surfaces/sessions/src/LedgerHead.tsx", { x: 900, y: 40, width: 400, height: 60 }, "Sessions"),
    note("20261007-135230-dddd", "The thread jumps when a message lands", "#session-thread", "div", "Thread", "packages/surfaces/sessions/src/Thread.tsx", { x: 300, y: 100, width: 600, height: 500 }, "Sessions", { status: "done" }),
    note("20261007-135400-eeee", "The repository path is cut off", "#settings-repo", "input", "RepositoryField", "packages/surfaces/settings/src/RepositoryField.tsx", { x: 300, y: 200, width: 500, height: 40 }, "Settings", {
      sent: { sessionId: "01SESSIONSENTEARLIER", title: "Repository field", at: AT },
    }),
    note("20261007-135500-ffff", "The window title is blank before the first Session", "#title", "h1", "TitleBar", "packages/shell/src/TitleBar.tsx", { x: 0, y: 0, width: 400, height: 30 }, null),
  ],
  // Eight notes on three screens: on the Cockpit two with no Job and one sent; on Sessions one with no Job, one sent and one
  // done; on Settings one with no Job and one done.
  "annotate-notes-tabs": () => [
    note("20261007-140000-aaaa", "The title is cut off in the row", 'section[aria-label="Jobs"]', "section", "Cockpit", "apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx", { x: 300, y: 120, width: 900, height: 200 }),
    note("20261007-140100-bbbb", "The places crowd the search field", 'nav[aria-label="Places"]', "nav", "TabBar", "apps/desktop/src/renderer/src/cockpit/deck/TabBar.tsx", { x: 0, y: 0, width: 220, height: 800 }),
    note("20261007-140200-cccc", "The branch chip overlaps the row badge", "#row-badge", "span", "RowBadge", "packages/surfaces/jobs/src/RowBadge.tsx", { x: 900, y: 140, width: 80, height: 20 }, "Cockpit", {
      sent: { sessionId: "01SESSIONSENTROWBADGEAAAAA", title: "Row badge spacing", at: AT },
    }),
    note("20261007-140300-dddd", "The ledger header wraps under the title", "#session-ledger-head", "header", "LedgerHead", "packages/surfaces/sessions/src/LedgerHead.tsx", { x: 900, y: 40, width: 400, height: 60 }, "Sessions"),
    note("20261007-140400-eeee", "The composer loses its draft on reload", "#session-composer", "form", "Composer", "packages/surfaces/sessions/src/Composer.tsx", { x: 300, y: 700, width: 600, height: 120 }, "Sessions", {
      sent: { sessionId: "01SESSIONSENTCOMPOSERAAAAA", title: "Composer draft", at: AT },
    }),
    note("20261007-140500-ffff", "The thread jumps when a message lands", "#session-thread", "div", "Thread", "packages/surfaces/sessions/src/Thread.tsx", { x: 300, y: 100, width: 600, height: 500 }, "Sessions", { status: "done" }),
    note("20261007-140600-gggg", "The repository path is cut off", "#settings-repo", "input", "RepositoryField", "packages/surfaces/settings/src/RepositoryField.tsx", { x: 300, y: 200, width: 500, height: 40 }, "Settings"),
    note("20261007-140700-hhhh", "The theme picker has no focus ring", "#settings-theme", "select", "ThemePicker", "packages/surfaces/settings/src/ThemePicker.tsx", { x: 300, y: 260, width: 300, height: 40 }, "Settings", { status: "done" }),
  ],
};

/** A one-pixel PNG, which is all a capture has to be here. */
const PNG = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="), (c) => c.charCodeAt(0)).buffer;

function sinkOf(notes: Annotation[]): Sink {
  const held = new Map(notes.map((one) => [one.id, one]));
  const shots = new Map<string, ArrayBuffer>();
  return {
    via: "main",
    list: async () => [...held.values()],
    save: async (one) => {
      held.set(one.id, one);
    },
    remove: async (id) => {
      held.delete(id);
      shots.delete(id);
    },
    root: async () => "/Users/user/code/armada",
    capture: async () => PNG,
    saveShot: async (id, png) => {
      shots.set(id, png);
    },
    shot: async (id) => shots.get(id) ?? null,
  };
}

/** Scenarios whose layer stays off until a person turns it on. */
const LAYER_OFF = new Set(["debug-bridge-draft", "report-from-right-click", "job-start-card-everywhere"]);

/** Whether the mock page draws this scenario's own layer, and so leaves the dev server's off. */
export const annotatesWith = (scenario: string): boolean => scenario in NOTES;

/** The layer over `scenario`'s notes, or nothing where it has none. Returns how to take it down. */
export function mountAnnotating(scenario: string): () => void {
  const notes = NOTES[scenario];
  if (notes === undefined) return () => undefined;
  try {
    // Off is written, not left unset: a window that walked a scenario with the layer on still holds its "1", and a layer
    // that opens on would swallow the right press this scenario's walk makes.
    sessionStorage.setItem(ON_KEY, LAYER_OFF.has(scenario) ? "0" : "1");
  } catch {
    // The layer then opens off, and a walk looking for its bar says so.
  }
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <WiredSessions>
        <Layer sink={sinkOf(notes())} />
      </WiredSessions>
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
