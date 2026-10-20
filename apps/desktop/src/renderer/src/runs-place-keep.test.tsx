// Keep on this Mac sits under a repository's places, and while it is on, On this Mac is the only place offered. Browser: what is drawn
// is read by role and name.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";

import { RunsPlaceCard } from "./RunsPlaceControl";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

function mount(kept: boolean, onKeep = vi.fn(async (_kept: boolean) => undefined as string | undefined)) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<RunsPlaceCard place={{ id: "mac" }} synced={[]} profiles={[]} onPick={async () => undefined} onFolder={async () => null} onDone={() => {}} kept={kept} onKeep={onKeep} />));
  roots.push({ root, host });
  return { host, onKeep };
}

const places = (host: HTMLElement) => [...host.querySelectorAll('[role="group"] .armada-runs__place-name')].map((one) => one.textContent);
const keep = (host: HTMLElement) => [...host.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')].find((one) => one.closest("label")?.textContent?.includes("Keep on this Mac"))!;

test("not kept, every place is offered and Keep on this Mac is off", () => {
  const { host } = mount(false);
  expect(places(host)).toEqual(["On this Mac", "In the repository", "iCloud or Google Drive", "Cloud storage"]);
  expect(keep(host).checked).toBe(false);
});

test("kept, only On this Mac is offered", () => {
  const { host } = mount(true);
  expect(places(host)).toEqual(["On this Mac"]);
  expect(keep(host).checked).toBe(true);
});

test("ticking it asks Fleet to keep the repository here, and a refusal is said in Fleet's words", async () => {
  const onKeep = vi.fn(async () => "Fleet does not serve /r");
  const { host } = mount(false, onKeep);
  await act(async () => keep(host).click());
  expect(onKeep).toHaveBeenCalledWith(true);
  expect(host.querySelector('[role="alert"]')?.textContent).toBe("Fleet does not serve /r");
});
