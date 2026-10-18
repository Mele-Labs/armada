// What a person can press, hover or tab to on the canvas map. The canvas takes no pointer; over it stands a layer of elements, one for each
// drone, bay, flag and crate, put where its picture is. They must be there to be found by name, reachable by keyboard, and say what they
// are when pressed or hovered. Browser: the layer is drawn on its own, over a stage that holds a view and draws nothing.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";

import { Hits, type HitActs } from "./Hits";
import { PANE, sceneFor } from "./scene-fixture";
import { viewOf } from "./project";
import type { Stage } from "./stage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mounted: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of mounted.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

/** A stage that holds the view the hit boxes are placed by and draws nothing. */
const stub = (): Stage => ({ shown: viewOf({ x: 0, y: 0, eye: 0.6, tilt: false }, PANE), onView: () => () => undefined, floorOf: () => 0 }) as unknown as Stage;

function draw(scene = sceneFor()) {
  const host = document.createElement("div");
  host.style.cssText = `position:relative;width:${PANE.width}px;height:${PANE.height}px`;
  document.body.append(host);
  const root = createRoot(host);
  const acts = { select: vi.fn(), pad: vi.fn(), goal: vi.fn(), pr: vi.fn(), tip: vi.fn() } satisfies HitActs;
  act(() => root.render(<Hits stage={stub()} scene={scene} facts={new Map()} acts={{ current: acts }} />));
  mounted.push({ root, host });
  return { host, acts };
}

const options = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>("[role=option]")];
const named = (host: HTMLElement, label: RegExp) => options(host).find((one) => label.test(one.getAttribute("aria-label") ?? ""))!;

test("the layer is the Map, and holds an option for each drone, bay, flag and crate, named as the card names it", () => {
  const { host } = draw();
  expect(host.querySelector("[role=listbox]")?.getAttribute("aria-label")).toBe("Map");
  const labels = options(host).map((one) => one.getAttribute("aria-label"));
  expect(labels).toEqual(
    expect.arrayContaining(["Writer, Job, Working", "Caller, Job, Needs you", "slot-1, r, Held", "slot-2, r, Held", "main in r: green, objective", "Pull request #7, Checks running"]),
  );
  // Nothing else is an option: one for each of the six.
  expect(labels).toHaveLength(6);
  // A drone says what it is; the others say where they are.
  expect(options(host).map((one) => one.getAttribute("data-hit")).sort()).toEqual(["drone", "drone", "goal", "pad", "pad", "payload"]);
});

test("each is reachable with Tab and placed over the canvas where the view puts it", () => {
  const { host } = draw();
  for (const one of options(host)) {
    expect(one.tabIndex, one.getAttribute("aria-label") ?? "").toBe(0);
    expect(one.style.display, one.getAttribute("aria-label") ?? "").toBe("");
    expect(one.style.transform).toMatch(/^translate\(/);
    expect(parseFloat(one.style.width)).toBeGreaterThan(0);
  }
});

test("a drone carries the call it holds as a button of its own, named for it", () => {
  const { host, acts } = draw();
  const call = host.querySelector<HTMLElement>("[data-hit=call]")!;
  expect(call.getAttribute("aria-label")).toBe("Caller needs you");
  expect(call.tabIndex).toBe(0);
  act(() => call.click());
  expect(acts.select).toHaveBeenCalledWith("session:Caller");
  expect(host.querySelectorAll("[data-hit=call]")).toHaveLength(1);
});

test("Enter on a drone selects it, and on a bay, the flag and a crate does what pressing them does", () => {
  const { host, acts } = draw();
  const enter = (one: HTMLElement) => act(() => void one.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
  enter(named(host, /^Writer,/));
  expect(acts.select).toHaveBeenCalledWith("session:Writer");
  enter(named(host, /^slot-2,/));
  expect(acts.pad).toHaveBeenCalledWith("slot-2");
  enter(named(host, /^main in r/));
  expect(acts.goal).toHaveBeenCalledWith("/r");
  enter(named(host, /^Pull request #7/));
  expect(acts.pr).toHaveBeenCalledWith("pr:7");
  // Space does the same; any other key does nothing.
  const before = acts.select.mock.calls.length;
  act(() => void named(host, /^Caller,/).dispatchEvent(new KeyboardEvent("keydown", { key: "a", bubbles: true, cancelable: true })));
  expect(acts.select.mock.calls.length).toBe(before);
  act(() => void named(host, /^Caller,/).dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true, cancelable: true })));
  expect(acts.select).toHaveBeenLastCalledWith("session:Caller");
});

test("hovering a drone shows its card, and leaving it takes the card away", () => {
  const { host, acts } = draw();
  const drone = named(host, /^Writer,/);
  // React's onMouseEnter listens to mouseover.
  act(() => void drone.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: document.body })));
  expect(acts.tip).toHaveBeenCalledTimes(1);
  const [tip, target, , lit] = acts.tip.mock.calls[0]!;
  expect(tip).toMatchObject({ label: "Job", title: "Writer" });
  expect(target).toBe(drone);
  // The drone it lights is the one it is about.
  expect(lit).toEqual({ hot: "session:Writer" });

  act(() => void drone.dispatchEvent(new MouseEvent("mouseout", { bubbles: true, relatedTarget: document.body })));
  expect(acts.tip).toHaveBeenLastCalledWith(undefined);
});

test("focusing a bay or the flag shows its card too, so a keyboard finds what a pointer does", () => {
  const { host, acts } = draw();
  act(() => named(host, /^slot-1,/).focus());
  expect(acts.tip.mock.calls.at(-1)![0]).toMatchObject({ label: "Bay", title: "slot-1" });
  act(() => named(host, /^main in r/).focus());
  expect(acts.tip.mock.calls.at(-1)![0]).toMatchObject({ label: "Main", title: "main in r" });
});

test("a drone outside the filters stands where it is and takes no pointer", () => {
  const scene = sceneFor();
  const dimmed = { ...scene, drones: scene.drones.map((one) => (one.drone.key === "session:Writer" ? { ...one, away: true } : one)) };
  const { host } = draw(dimmed);
  expect(named(host, /^Writer,/).hasAttribute("data-away")).toBe(true);
  expect(named(host, /^Caller,/).hasAttribute("data-away")).toBe(false);
});
