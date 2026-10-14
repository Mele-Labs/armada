// The Cockpit window's tabs and its strip of system pages, drawn and pressed, and the digits that reach
// the tabs. Browser: a press and a rendering are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { SURFACE, useSurfaceKeys } from "@armada/shell";

import { SystemStrip } from "./SystemStrip";
import { TabBar } from "./TabBar";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

function mount(node: React.ReactNode): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(node));
  roots.push({ root, host });
  return host;
}

const tab = (host: HTMLElement, name: string) => [...host.querySelectorAll<HTMLElement>('[role="tab"]')].find((one) => one.textContent?.startsWith(name))!;

test("The tab bar maps each tab to its surface, and System opens the strip", () => {
  const went: string[] = [];
  const bar = mount(<TabBar showing={SURFACE.overview} onGoTo={(id) => went.push(id)} />);
  expect([...bar.querySelectorAll('[role="tab"]')].map((one) => one.textContent)).toEqual(["Cockpit", "Jobs", "Sessions", "System"]);
  expect(tab(bar, "Cockpit").getAttribute("aria-selected")).toBe("true");
  for (const name of ["Jobs", "Sessions", "System"]) act(() => tab(bar, name).click());
  expect(went).toEqual([SURFACE.jobs, SURFACE.sessions, SURFACE.kit]);

  // The strip is there when System is the place, whichever of its pages is up, and is not otherwise.
  expect(mount(<SystemStrip showing={SURFACE.overview} onGoTo={() => {}} />).querySelector("nav")).toBeNull();
  expect(mount(<SystemStrip showing={SURFACE.jobs} onGoTo={() => {}} />).querySelector("nav")).toBeNull();
  const strip = mount(<SystemStrip showing={SURFACE.mods} onGoTo={(id) => went.push(id)} />);
  expect(strip.querySelector("nav")?.getAttribute("aria-label")).toBe("System");
  expect([...strip.querySelectorAll('[role="tab"]')].map((one) => one.textContent)).toEqual(["Kit", "Mods", "Settings", "Guides", "Workflows", "Doctor", "Studios", "Retros"]);
  expect(tab(strip, "Mods").getAttribute("aria-selected")).toBe("true");
  act(() => tab(strip, "Retros").click());
  expect(went.at(-1)).toBe(SURFACE.lessons);
});

test("The Cockpit tab keeps its pip while a call waits elsewhere, and does not draw one otherwise", () => {
  const away = mount(<TabBar showing={SURFACE.jobs} waiting onGoTo={() => {}} />);
  expect(away.querySelector('[role="img"][aria-label="A call is waiting"]')).not.toBeNull();
  expect(tab(away, "Cockpit").querySelector('[role="img"]')).not.toBeNull();
  expect(tab(away, "Jobs").querySelector('[role="img"]')).toBeNull();
  expect(mount(<TabBar showing={SURFACE.jobs} onGoTo={() => {}} />).querySelector('[role="img"]')).toBeNull();
});

function Keys({ deck, onSurface }: { deck: boolean; onSurface: (id: string) => void }) {
  useSurfaceKeys(onSurface, deck);
  return null;
}
const cmd = (digit: string) => act(() => void window.dispatchEvent(new KeyboardEvent("keydown", { key: digit, metaKey: true, bubbles: true, cancelable: true })));

test("Cmd+1..4 switch tabs only when the rail is off", () => {
  const off: string[] = [];
  mount(<Keys deck onSurface={(id) => off.push(id)} />);
  for (const digit of ["1", "2", "3", "4", "5"]) cmd(digit);
  // ⌘5 is the rail's own, Manifest, in either window.
  expect(off).toEqual([SURFACE.overview, SURFACE.jobs, SURFACE.sessions, SURFACE.kit, SURFACE.manifest]);
});

test("With the rail on, the digits are the rail's order and no tab's", () => {
  const on: string[] = [];
  mount(<Keys deck={false} onSurface={(id) => on.push(id)} />);
  for (const digit of ["1", "2", "3", "4", "5"]) cmd(digit);
  // Cockpit, Studios, then Alerts, which is not built and so reaches nothing, Doctor and Manifest.
  expect(on).toEqual([SURFACE.overview, SURFACE.studios, SURFACE.doctor, SURFACE.manifest]);
});
