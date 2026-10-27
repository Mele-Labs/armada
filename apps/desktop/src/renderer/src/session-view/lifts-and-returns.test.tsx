// The subagents that lift off and fly back on a Job's Field (`useLiftsAndReturns`), across the person opening another Job. Browser: a hook is read through a rendering.
// Motion is asked for here: the hook plays nothing under reduced motion, which is the one setting this was never seen under.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { DroneIn } from "../cockpit/deck/layout";
import { useLiftsAndReturns } from "./field-now";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const drone = (key: string, parent?: string): DroneIn => ({ key, kind: "session", title: key, root: "/r", said: "Working", hue: "calm", spin: true, hover: true, ping: false, stuck: false, flare: false, working: true, touches: [], shown: true, ...(parent === undefined ? {} : { parent }) });

let seen: readonly DroneIn[] = [];
function Probe({ job, drones }: { job: string; drones: readonly DroneIn[] }) {
  seen = useLiftsAndReturns(job, drones, true).drones;
  return null;
}

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => {
  vi.spyOn(window, "matchMedia").mockImplementation(() => ({ matches: false }) as MediaQueryList);
});
afterEach(() => {
  vi.restoreAllMocks();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  return (job: string, drones: readonly DroneIn[]) => act(() => root.render(<Probe job={job} drones={drones} />));
}

test("a subagent that ends flies back and stays drawn until it has, once", () => {
  const show = mount();
  const own = drone("session:a");
  const sub = drone("session:a/x", "session:a");
  show("a", [own, sub]);
  show("a", [own]);
  expect(seen.map((one) => one.key)).toEqual(["session:a", "session:a/x"]);
});

test("opening another Job does not fly the first Job's subagents back, since they are drawn as that Job's drones now", () => {
  const show = mount();
  show("a", [drone("session:a"), drone("session:a/x", "session:a")]);
  // The other Job has a subagent of its own, which is what makes the hook look for what went.
  show("b", [drone("session:b"), drone("session:b/y", "session:b")]);
  expect(seen.map((one) => one.key)).toEqual(["session:b", "session:b/y"]);
  expect(new Set(seen.map((one) => one.key)).size).toBe(seen.length);
});
