// When the canvas draws. **A frame is drawn when something changed and not otherwise**: an unchanged scene draws nothing, a map nobody is
// touching has its ticker stopped, a loop or a play starts it and it stops itself, and a window nobody is looking at (`data-still`,
// `data-idle`) holds with the pose as it is. Browser: the stage is the real one on a real WebGL canvas; the test reads its application's
// ticker and counts the frames the application draws.

import type { Application } from "pixi.js";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { commands } from "vitest/browser";

import "../../../styles/index.css";
import { PANE, sceneFor } from "./scene-fixture";
import { Stage } from "./stage";

const CAM = { x: 0, y: 0, eye: 1, tilt: false };
const wait = (ms: number) => new Promise((done) => window.setTimeout(done, ms));

let host: HTMLElement;
let stage: Stage;
const app = () => (stage as unknown as { app: Application | undefined }).app!;
const started = () => app().ticker.started;
/** The frames the application has drawn since this was called. */
const counting = () => {
  // The renderer, not `app.render`: the ticker holds the application's own method from when it started.
  const render = vi.spyOn(app().renderer, "render");
  render.mockClear();
  return () => render.mock.calls.length;
};

beforeEach(async () => {
  // This project runs with reduced motion, which draws no loop at all; the stage reads it, and the tokens' durations, when it starts.
  await commands.motion(true);
  host = document.createElement("div");
  host.style.cssText = `position:relative;width:${PANE.width}px;height:${PANE.height}px`;
  document.body.append(host);
  stage = Stage.start(host);
  await expect.poll(() => host.querySelector("canvas") !== null, { timeout: 60_000 }).toBe(true);
  stage.setSize(PANE);
  stage.setCamera(CAM, true);
});

afterEach(async () => {
  document.documentElement.removeAttribute("data-idle");
  document.documentElement.removeAttribute("data-still");
  stage.destroy();
  host.remove();
  await commands.motion(false);
});

/** A scene whose Session `Writer` is running a tool over a plot: one of the drones the map animates. */
const working = () => {
  const scene = sceneFor();
  const tile = scene.layout.territories[0]!.tiles[0]!;
  return { ...scene, drones: scene.drones.map((one) => (one.drone.key === "session:Writer" ? { ...one, loop: true, aim: { tile: tile.path, at: { x: tile.cx, y: tile.cy }, wide: 40, deep: 20 } } : one)) };
};

test("a map nobody is touching has its ticker stopped, and an unchanged scene draws nothing", async () => {
  stage.setScene(sceneFor());
  await wait(300);
  expect(started()).toBe(false);

  const drawn = counting();
  // The page hands a fresh scene every five seconds with the same content in it.
  stage.setScene(sceneFor());
  stage.setScene(sceneFor());
  await wait(300);
  expect(drawn()).toBe(0);
  expect(started()).toBe(false);

  // One that changed is drawn once, on the next animation frame, and the ticker stays stopped.
  stage.setScene(sceneFor({ hotTile: sceneFor().layout.territories[0]!.tiles[0]!.path }));
  await expect.poll(drawn).toBe(1);
  await wait(200);
  expect(drawn()).toBe(1);
  expect(started()).toBe(false);
});

test("a drone that works starts the ticker, and it stops itself once nothing loops", async () => {
  stage.setScene(working());
  await expect.poll(started).toBe(true);
  const drawn = counting();
  // Frames keep coming, not one: a loop is drawn on the ticker, at thirty a second at most.
  await expect.poll(drawn).toBeGreaterThan(2);

  stage.setScene(sceneFor());
  await expect.poll(started, { timeout: 5_000 }).toBe(false);
  const after = counting();
  await wait(300);
  expect(after()).toBe(0);
});

test("a window nobody is looking at holds: the ticker stops and a changed scene is not drawn until it is looked at again", async () => {
  stage.setScene(working());
  await expect.poll(started).toBe(true);

  document.documentElement.setAttribute("data-idle", "");
  await expect.poll(started).toBe(false);
  const drawn = counting();
  stage.setScene(sceneFor({ hotTile: sceneFor().layout.territories[0]!.tiles[0]!.path }));
  stage.setScene(working());
  await wait(300);
  expect(drawn()).toBe(0);
  expect(started()).toBe(false);

  // Looked at again: the loop that was held runs.
  document.documentElement.removeAttribute("data-idle");
  await expect.poll(started).toBe(true);
});

test("hidden (`data-still`) holds the same way", async () => {
  stage.setScene(working());
  await expect.poll(started).toBe(true);
  document.documentElement.setAttribute("data-still", "");
  await expect.poll(started).toBe(false);
  document.documentElement.removeAttribute("data-still");
  await expect.poll(started).toBe(true);
});

test("a camera that moves by hand is drawn once for each move and then the ticker is stopped", async () => {
  stage.setScene(sceneFor());
  await wait(200);
  const drawn = counting();
  stage.setCamera({ ...CAM, x: 120 }, true);
  await expect.poll(drawn).toBeGreaterThan(0);
  await wait(200);
  expect(started()).toBe(false);
});

test("a folded pane draws nothing and builds nothing, and draws what it was handed once it is shown", async () => {
  stage.setScene(working());
  await expect.poll(started).toBe(true);
  stage.setHidden(true);
  await expect.poll(started).toBe(false);
  const drawn = counting();
  const built = vi.spyOn(stage as unknown as { sync: (force: boolean) => void }, "sync");
  stage.setScene(sceneFor({ hotTile: sceneFor().layout.territories[0]!.tiles[0]!.path }));
  stage.setScene(working());
  await wait(300);
  expect(drawn()).toBe(0);
  expect(built).not.toHaveBeenCalled();
  stage.setHidden(false);
  expect(built).toHaveBeenCalledTimes(1);
  await expect.poll(started).toBe(true);
});

test("a theme put on, or fonts arriving, while the pane is folded builds nothing until it is shown, and then draws everything again", async () => {
  stage.setScene(working());
  await expect.poll(started).toBe(true);
  stage.setHidden(true);
  await expect.poll(started).toBe(false);
  const built = vi.spyOn(stage as unknown as { sync: (force: boolean) => void }, "sync");
  const root = document.documentElement;
  const theme = root.getAttribute("data-theme");
  root.setAttribute("data-theme", theme ?? "dark");
  await wait(100);
  expect(built).not.toHaveBeenCalled();
  stage.setHidden(false);
  expect(built).toHaveBeenCalledWith(true);
  if (theme === null) root.removeAttribute("data-theme");
});

test("a glow or a bar that moves draws the bars again and leaves the plots as they were", async () => {
  const scene = sceneFor();
  stage.setScene(scene);
  await wait(200);
  const layers = stage as unknown as { tiles: { set: () => void }; marks: { set: () => void } };
  const [plots, bars] = [vi.spyOn(layers.tiles, "set"), vi.spyOn(layers.marks, "set")];
  const path = scene.layout.territories[0]!.tiles[0]!.path;
  stage.setScene({ ...scene, glow: new Map([[`${path}/lib.rs`, { act: "write", v: 0.5 }]]), touched: new Set([path]) });
  await wait(200);
  expect(bars).toHaveBeenCalledTimes(1);
  expect(plots).not.toHaveBeenCalled();
});

test("a drone handed twice in one scene is drawn once, and the map keeps drawing", async () => {
  const scene = sceneFor();
  stage.setScene(scene);
  await wait(200);
  const writer = scene.drones.find((one) => one.drone.key === "session:Writer")!;
  // A subagent that left twice within its return: the second copy as it was drawn, the first changed.
  const twice = { ...scene, drones: [...scene.drones.filter((one) => one !== writer), { ...writer, selected: true }, writer] };
  expect(() => stage.setScene(twice)).not.toThrow();
  expect(stage.drones.children.length).toBe(scene.drones.length);
});
