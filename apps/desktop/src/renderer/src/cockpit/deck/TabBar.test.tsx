// The window's tabs, drawn and pressed, and the digits that reach
// the tabs. Browser: a press and a rendering are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { SURFACE, useSurfaceKeys } from "@armada/shell";

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

const tab = (host: HTMLElement, name: string, nth = 0) => [...host.querySelectorAll<HTMLElement>('[role="tab"]')].filter((one) => one.textContent?.startsWith(name))[nth]!;

test("The tab bar maps each tab to its surface", () => {
  const went: string[] = [];
  const bar = mount(<TabBar showing={SURFACE.overview} onGoTo={(id) => went.push(id)} />);
  expect([...bar.querySelectorAll('[role="tab"]')].map((one) => one.textContent)).toEqual(["Cockpit", "Jobs", "Jobs", "Studios", "Retros", "Reviews", "System"]);
  expect(tab(bar, "Cockpit").getAttribute("aria-selected")).toBe("true");
  const pressed = ["Jobs", "Jobs", "Studios", "Retros", "Reviews", "System"];
  pressed.forEach((name, at) => act(() => tab(bar, name, pressed.slice(0, at).filter((one) => one === name).length).click()));
  expect(went).toEqual([SURFACE.jobs, SURFACE.sessions, SURFACE.studios, SURFACE.lessons, SURFACE.reviews, SURFACE.kit]);
});

test("A Studio's pages and the Retros page are under their own tabs", () => {
  expect(tab(mount(<TabBar showing={SURFACE.studios} onGoTo={() => {}} />), "Studios").getAttribute("aria-selected")).toBe("true");
  expect(tab(mount(<TabBar showing={SURFACE.lessons} onGoTo={() => {}} />), "Retros").getAttribute("aria-selected")).toBe("true");
});

test("The Cockpit tab keeps its pip while a call waits elsewhere, and does not draw one otherwise", () => {
  const away = mount(<TabBar showing={SURFACE.jobs} waiting onGoTo={() => {}} />);
  expect(away.querySelector('[role="img"][aria-label="A call is waiting"]')).not.toBeNull();
  expect(tab(away, "Cockpit").querySelector('[role="img"]')).not.toBeNull();
  expect(tab(away, "Jobs").querySelector('[role="img"]')).toBeNull();
  expect(mount(<TabBar showing={SURFACE.jobs} onGoTo={() => {}} />).querySelector('[role="img"]')).toBeNull();
});

test("The Cockpit tab pressed while it is the place goes nowhere, and goes back to it while a page opened from it is over it; Jobs and Sessions go up to their list", () => {
  const went: string[] = [];
  const here = mount(<TabBar showing={SURFACE.overview} onGoTo={(id) => went.push(id)} />);
  act(() => tab(here, "Cockpit").click());
  expect(went).toEqual([]);
  const over = mount(<TabBar showing={SURFACE.overview} away onGoTo={(id) => went.push(id)} />);
  act(() => tab(over, "Cockpit").click());
  expect(went).toEqual([SURFACE.overview]);
  const jobs = mount(<TabBar showing={SURFACE.jobs} onGoTo={(id) => went.push(id)} />);
  act(() => tab(jobs, "Jobs").click());
  expect(went).toEqual([SURFACE.overview, SURFACE.jobs]);
});

function Keys({ onSurface }: { onSurface: (id: string) => void }) {
  useSurfaceKeys(onSurface);
  return null;
}
const cmd = (digit: string) => act(() => void window.dispatchEvent(new KeyboardEvent("keydown", { key: digit, metaKey: true, bubbles: true, cancelable: true })));

test("Cmd+1..6 open the tabs in the bar's order and no other digit reaches a place; Cmd+, opens Settings and Cmd+5 is Retros", () => {
  const went: string[] = [];
  mount(<Keys onSurface={(id) => went.push(id)} />);
  for (const digit of ["1", "2", "3", "4", "5", "6", "7", "8", "9"]) cmd(digit);
  expect(went).toEqual([SURFACE.overview, SURFACE.jobs, SURFACE.sessions, SURFACE.studios, SURFACE.lessons, SURFACE.kit]);
  went.length = 0;
  cmd(",");
  expect(went).toEqual([SURFACE.settings]);
});
