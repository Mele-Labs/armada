// A Session's Field has a camera of its own. The window's kept camera is the Cockpit map's, laid on a plane of another size, and
// held to this one it lands at the lower right corner; the Field opens on the whole floor, and keeps a zoom he makes until it is
// given another `own`. Browser: the camera is read off where the drone's hit box stands over the canvas, which the stage puts by the view it draws.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import { Battlefield } from "./Battlefield";
import type { DroneIn, RepoIn } from "./layout";
import { camera } from "./store";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const NONE = new Map();
const repos: RepoIn[] = [{ root: "/r", name: "r", pads: [], goal: false, areas: [{ dir: "crates", kids: [["fleet", 40], ["store", 20]] }, { dir: "apps", kids: [["desktop", 60]] }] }];
const drones: DroneIn[] = [{ key: "session:a", kind: "session", title: "a", root: "/r", said: "Working", hue: "calm", spin: false, hover: true, ping: false, stuck: false, flare: false, working: false, touches: [], shown: true }];

/** The pane the Field is drawn in, in pixels. */
const PANE = { width: 640, height: 360 };

const mounted: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of mounted.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  camera.current = undefined;
});

function draw(own: string | undefined) {
  const host = document.createElement("div");
  host.style.position = "relative";
  host.style.width = `${PANE.width}px`;
  host.style.height = `${PANE.height}px`;
  document.body.append(host);
  const root = createRoot(host);
  const render = (key: string | undefined) =>
    act(() =>
      root.render(
        <Battlefield repos={repos} drones={drones} pads={NONE} goals={NONE} prs={[]} trails={NONE} plays={[]} facts={NONE} calls={NONE} clashes={[]} selected={undefined} own={key} onSelect={() => undefined} onPad={() => undefined} onGoal={() => undefined} onPr={() => undefined} />,
      ),
    );
  render(own);
  mounted.push({ root, host });
  return { host, render };
}

const seen = (host: HTMLElement) => {
  const one = host.querySelector<HTMLElement>("[data-hit=drone]")!;
  return `${one.style.transform} ${one.style.width}`;
};
// The canvas starts a moment after it is mounted and then eases to the camera it is handed.
const settle = () => act(() => new Promise<void>((done) => window.setTimeout(done, 1200)));

test("a Field opens on the whole floor, whatever camera the window kept", async () => {
  const fresh = draw("a");
  await settle();
  const whole = seen(fresh.host);

  camera.current = { x: 9000, y: 9000, eye: 2.4, tilt: true };
  const stale = draw("a");
  await settle();
  expect(seen(stale.host)).toBe(whole);

  // The window's own map takes the kept camera, which is the cause: the same plane held to it lands elsewhere.
  const map = draw(undefined);
  await settle();
  expect(seen(map.host)).not.toBe(whole);
});

test("a zoom stays until the Field is given another Session, and then it is the whole floor again", async () => {
  const field = draw("a");
  await settle();
  const whole = seen(field.host);

  act(() => {
    field.host.querySelector(".bf")!.dispatchEvent(new WheelEvent("wheel", { deltaY: -240, ctrlKey: true, clientX: PANE.width / 2, clientY: PANE.height / 2, bubbles: true, cancelable: true }));
  });
  await settle();
  const zoomed = seen(field.host);
  expect(zoomed).not.toBe(whole);

  field.render("a");
  await settle();
  expect(seen(field.host)).toBe(zoomed);

  field.render("b");
  await settle();
  expect(seen(field.host)).toBe(whole);
});
