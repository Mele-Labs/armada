// Mods made: the Landing panel of a Mod Job lists the mods Fleet's list says it made (`made_by`), and its switch is the one the Mods page uses.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { ModSummary, Outcome } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";
import { NOTHING_YET } from "../../../shared/bridge";
import type { StateChange } from "../../../shared/state-change";
import { ModsMade } from "./ModsPanel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  Object.defineProperty(window, "armada", { value: undefined, configurable: true });
});

const row = (name: string, extra: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "theme", enabled: true, valid: true, ...extra });

/** A Fleet whose mod list is `mods`, and which answers every switch with `outcome`. */
function fleet(mods: ModSummary[], outcome: Outcome = { ok: true }) {
  let push: (change: StateChange<BridgeState>) => void = () => undefined;
  const setModEnabled = vi.fn(async (_name: string, _enabled: boolean): Promise<Outcome> => outcome);
  const api = {
    syncState: async () => ({ full: { ...NOTHING_YET, mods: { mods } }, seq: 1 }),
    subscribeChanges: (on: (change: StateChange<BridgeState>) => void) => {
      push = on;
      return () => undefined;
    },
    setModEnabled,
  };
  Object.defineProperty(window, "armada", { value: api, configurable: true });
  return { setModEnabled, listed: (next: ModSummary[]) => push({ full: { ...NOTHING_YET, mods: { mods: next } }, seq: 2 }) };
}

async function mount(id: string): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<ModsMade id={id} />));
  roots.push({ root, host });
  return host;
}

const switches = (host: HTMLElement) => [...host.querySelectorAll<HTMLInputElement>('[role="switch"]')].map((one) => one.getAttribute("aria-label") ?? "");

test("lists the mods whose made_by is this Job and no others", async () => {
  fleet([row("dusk", { made_by: "job-1" }), row("by-another", { made_by: "job-2" }), row("by-hand"), row("built-in:calm", { origin: "built_in" })]);
  const host = await mount("job-1");
  expect(switches(host)).toEqual(["Dusk"]);
});

test("a mod Fleet lists later joins the panel", async () => {
  const fake = fleet([]);
  const host = await mount("job-1");
  expect(switches(host)).toEqual([]);
  await act(async () => fake.listed([row("dusk", { made_by: "job-1" })]));
  expect(switches(host)).toEqual(["Dusk"]);
});

test("its switch asks Fleet to turn the mod off, as the Mods page does", async () => {
  const { setModEnabled } = fleet([row("dusk", { made_by: "job-1" })]);
  const host = await mount("job-1");
  await act(async () => host.querySelector<HTMLElement>('[role="switch"]')!.click());
  expect(setModEnabled).toHaveBeenCalledWith("dusk", false);
});

test("a switch Fleet refuses is told, in the words the Mods page uses", async () => {
  const told: string[] = [];
  const hear = (event: Event): void => void told.push((event as CustomEvent<{ sentence: string }>).detail.sentence);
  window.addEventListener("armada:tell", hear);
  try {
    fleet([row("dusk", { made_by: "job-1" })], { ok: false, why: "not_connected" });
    const host = await mount("job-1");
    await act(async () => host.querySelector<HTMLElement>('[role="switch"]')!.click());
    await vi.waitFor(() => expect(told).toHaveLength(1));
  } finally {
    window.removeEventListener("armada:tell", hear);
  }
});
