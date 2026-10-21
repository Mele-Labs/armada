// The Bays panel adds a slot to a repository's pool from its heading, on the act Cleanup's add tile sends, and says a refusal under it.
// Browser: the press goes through the rendered button.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import type { Outcome, WorktreeSlot } from "@armada/protocol";

import { Bays } from "./BayRows";
import { bayViewOf } from "./bays";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

/** A free bay of each repository named, so each draws a heading. */
async function mountWith(repos: { manifest: string }[], onAddSlot?: (manifest: string) => Promise<Outcome>) {
  const views = repos.map(({ manifest }) => {
    const slot = { manifest_id: manifest, slot: 1, path: `/x/${manifest}/slot-1`, base: "main", warm: true, behind: 0, held: { state: "free" } } as WorktreeSlot;
    return bayViewOf(slot, `${manifest}:1`, { leases: [], first: undefined }, undefined, Date.now());
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  await act(async () => {
    root.render(<Bays views={views} repoName={(manifest) => `repo ${manifest}`} working={new Set()} selected={undefined} hovered={undefined} onHover={() => {}} onSelect={() => {}} onAddSlot={onAddSlot} />);
  });
  return host;
}

const press = async (host: HTMLElement, label: string) => {
  const button = host.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`);
  expect(button).not.toBeNull();
  await act(async () => button!.click());
};

test("the heading's button adds a slot to that repository's pool", async () => {
  const asked: string[] = [];
  const host = await mountWith([{ manifest: "m1" }], async (manifest) => {
    asked.push(manifest);
    return { ok: true } as Outcome;
  });
  await press(host, "Add a slot to repo m1");
  expect(asked).toEqual(["m1"]);
  expect(host.querySelector('[role="alert"]')).toBeNull();
});

test("a refusal is said under the heading", async () => {
  const host = await mountWith([{ manifest: "m1" }], async () => ({ ok: false, code: "fleet.slot_pool_change_refused", message: "The pool is at its ceiling" }) as unknown as Outcome);
  await press(host, "Add a slot to repo m1");
  expect(host.querySelector('[role="alert"]')?.textContent).toMatch(/^Slot not added: /);
});

test("with no route to change the pool, no button is drawn", async () => {
  const host = await mountWith([{ manifest: "m1" }]);
  expect(host.querySelector('[aria-label="Add a slot to repo m1"]')).toBeNull();
});

test("while an add is out the button shows busy and the heading says so", async () => {
  let settle: (outcome: Outcome) => void = () => {};
  const host = await mountWith([{ manifest: "m1" }], () => new Promise<Outcome>((resolve) => (settle = resolve)));
  await press(host, "Add a slot to repo m1");
  const button = host.querySelector<HTMLButtonElement>('[aria-label="Add a slot to repo m1"]');
  expect(button?.getAttribute("aria-busy")).toBe("true");
  expect(button?.getAttribute("data-pending")).toBe("true");
  expect(host.querySelector('[role="status"]')?.textContent).toBe("Adding a slot…");
  await act(async () => settle({ ok: true } as Outcome));
  expect(button?.getAttribute("aria-busy")).toBeNull();
});

test("a slot added is said under the heading until the next press", async () => {
  const answers: Outcome[] = [{ ok: true } as Outcome, { ok: false, code: "fleet.slot_pool_change_refused", message: "The pool is at its ceiling" } as unknown as Outcome];
  const host = await mountWith([{ manifest: "m1" }], async () => answers.shift()!);
  await press(host, "Add a slot to repo m1");
  expect(host.querySelector('[role="status"]')?.textContent).toBe("Slot added");
  await press(host, "Add a slot to repo m1");
  expect(host.querySelector('[role="status"]')).toBeNull();
  expect(host.querySelector('[role="alert"]')?.textContent).toMatch(/^Slot not added: /);
});
