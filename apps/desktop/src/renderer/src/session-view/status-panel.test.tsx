// The Status panel's steps: the one Fleet names is lit, the ones before it are done, and with none named every step is to come.
// Browser: a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import type { SessionWork } from "@armada/screens/src/draft/sessions";

import { StatusPanel } from "./StatusPanel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

function mount(work: SessionWork): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<StatusPanel work={work} place="side" id="s1" acts={undefined} />));
  roots.push({ root, host });
  return host;
}

const states = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>(".sv-step")].map((one) => one.dataset["state"]);

test("the step Fleet names is lit, the ones before it are done and the ones after are to come", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan", "Build", "Pull request"], at: 1 } });
  expect(states(host)).toEqual(["done", "now", "next"]);
  expect(host.querySelector('[aria-current="step"]')?.textContent).toContain("Build");
});

test("with no step named every step is to come and none is lit", () => {
  const host = mount({ workflow: { name: "Feature", steps: ["Plan", "Build"] } });
  expect(states(host)).toEqual(["next", "next"]);
  expect(host.querySelector('[aria-current="step"]')).toBeNull();
});
