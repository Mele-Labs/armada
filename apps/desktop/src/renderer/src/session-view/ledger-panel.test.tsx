// The Ledger panel of a Session viewed on the Cockpit: what the Session holds beside its branch and bay is listed, and with nothing
// else held the panel is its band, with no placeholder rows, and opens on its own once something arrives. Browser: a rendering is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import type { LedgerEntry } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { LedgerPanel } from "./LedgerPanel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => localStorage.clear());
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  localStorage.clear();
});

const session = { id: "s1", attachments: [{ kind: "slot", slot: 9 }, { kind: "branch", name: "bridge/queue-for-later", slot: 9 }], rows: [], turn: { state: "idle" } } as unknown as Session;
const entry = (kind: LedgerEntry["kind"], key: string, text: string): LedgerEntry => ({ key, kind, name: text, text, onOpen: () => undefined, ...(kind === "artifact" ? { artifact: "window" } : {}) }) as unknown as LedgerEntry;
const HELD = [entry("slot", "slot9", "9"), entry("branch", "bridge/queue-for-later", "bridge/queue-for-later")];

function mount(entries: readonly LedgerEntry[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const draw = (next: readonly LedgerEntry[]) => act(() => root.render(<LedgerPanel session={session} entries={next} place="side" />));
  draw(entries);
  roots.push({ root, host });
  return { host, draw };
}

const panel = (host: HTMLElement) => host.querySelector<HTMLElement>('section[aria-label="Ledger"]')!;

test("it lists what the Session holds beside its branch and bay, which are chips in the band", () => {
  const { host } = mount([...HELD, entry("artifact", "w1", "Queue for later walk"), entry("subagent", "a1", "Mock walk subagent")]);
  expect(host.querySelectorAll('section[aria-label="Ledger"]')).toHaveLength(1);
  expect(panel(host).hasAttribute("data-collapsed")).toBe(false);
  expect(panel(host).textContent).toContain("Queue for later walk");
  expect(panel(host).textContent).toContain("Mock walk subagent");
  expect(host.querySelector('[aria-label="Branch bridge/queue-for-later"]')).not.toBeNull();
  expect(host.querySelector('[aria-label="Bay 9"]')).not.toBeNull();
});

test("with only a slot and a branch held it is its band: folded, no placeholder, the chips kept", () => {
  const { host } = mount(HELD);
  expect(panel(host).hasAttribute("data-collapsed")).toBe(true);
  expect(panel(host).hasAttribute("data-empty")).toBe(true);
  expect(host.querySelector(".armada-session-ledger")).toBeNull();
  expect(host.querySelector('[aria-label="Bay 9"]')).not.toBeNull();
});

test("it opens on its own when an entry arrives, and a press on the band opens and folds it while it is empty", () => {
  const { host, draw } = mount(HELD);
  const band = panel(host).querySelector<HTMLElement>("header")!;
  act(() => band.click());
  expect(panel(host).hasAttribute("data-collapsed")).toBe(false);
  act(() => band.click());
  expect(panel(host).hasAttribute("data-collapsed")).toBe(true);
  draw([...HELD, entry("pull_request", "2430", "#2430 Ledger rows")]);
  expect(panel(host).hasAttribute("data-collapsed")).toBe(false);
  expect(panel(host).textContent).toContain("#2430 Ledger rows");
});
