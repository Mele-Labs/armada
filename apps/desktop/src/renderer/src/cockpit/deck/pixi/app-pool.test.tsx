// Which application a map draws on. Browser: the stages are real ones on real WebGL canvases; the test reads their applications.

import type { Application } from "pixi.js";
import { afterEach, beforeEach, expect, test } from "vitest";
import { commands } from "vitest/browser";

import "../../../styles/index.css";
import { PANE } from "./scene-fixture";
import { idleApps } from "./app-pool";
import { Stage } from "./stage";

const appOf = (stage: Stage) =>
  (
    stage as unknown as {
      app: Application | undefined;
    }
  ).app;

let hosts: HTMLElement[] = [];
let stages: Stage[] = [];

/** A stage in a host of its own, once its canvas is in. */
const open = async (): Promise<{
  host: HTMLElement;
  stage: Stage;
  app: Application;
}> => {
  const host = document.createElement("div");
  host.style.cssText = `position:relative;width:${PANE.width}px;height:${PANE.height}px`;
  document.body.append(host);
  hosts.push(host);
  const stage = Stage.start(host);
  stages.push(stage);
  await expect
    .poll(() => host.querySelector("canvas") !== null, {
      timeout: 60_000,
    })
    .toBe(true);
  return {
    host,
    stage,
    app: appOf(stage)!,
  };
};

beforeEach(async () => {
  await commands.motion(true);
});

afterEach(async () => {
  for (const stage of stages) stage.destroy();
  for (const host of hosts) host.remove();
  stages = [];
  hosts = [];
  for (const app of idleApps().splice(0))
    app.destroy(
      {
        removeView: true,
      },
      {
        children: true,
      },
    );
  await commands.motion(false);
});

test("a map that starts after another ended draws on the same application, with its canvas moved and nothing left behind", async () => {
  const first = await open();
  first.stage.destroy();
  expect(first.host.querySelector("canvas")).toBeNull();
  expect(idleApps()).toEqual([
    first.app,
  ]);
  expect(first.app.stage.children).toHaveLength(0);
  expect(first.app.ticker.started).toBe(false);

  const second = await open();
  expect(second.app).toBe(first.app);
  expect(idleApps()).toHaveLength(0);
  expect(second.host.querySelectorAll("canvas")).toHaveLength(1);
  expect(second.host.querySelector("canvas")).toBe(first.app.canvas);
});

test("two maps live at once draw on two applications, and both come back for the next two", async () => {
  const a = await open();
  const b = await open();
  expect(a.app).not.toBe(b.app);
  a.stage.destroy();
  b.stage.destroy();
  expect(idleApps()).toHaveLength(2);

  const c = await open();
  const d = await open();
  expect(
    new Set([
      c.app,
      d.app,
    ]),
  ).toEqual(
    new Set([
      a.app,
      b.app,
    ]),
  );
});

test("an application whose context was lost is dropped, not handed on", async () => {
  const first = await open();
  const lose = (first.app.canvas as HTMLCanvasElement)
    .getContext("webgl2")
    ?.getExtension("WEBGL_lose_context");
  expect(lose).toBeTruthy();
  lose!.loseContext();
  first.stage.destroy();
  expect(idleApps()).toHaveLength(0);

  const second = await open();
  expect(second.app).not.toBe(first.app);
});
