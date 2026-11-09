// The Pixi applications the maps draw on, kept between maps. **A WebGL context is made once and handed on**: opening a Job destroys the
// Cockpit's map and starts the Job's own, and a new application each time costs a shader compile and a teardown per open, and Chromium
// allows about sixteen live contexts to a page. A map that ends gives its application back clean; the next map takes it from here.

import { Application, RendererType, Ticker } from "pixi.js";

import type { Size } from "../layout";

/** Idle applications kept: two maps are live at most, and each idle one holds a context. */
const IDLE_MAX = 2;
const idle: Application[] = [];

const resolutionNow = () => Math.min(window.devicePixelRatio || 1, 2);

const lost = (app: Application): boolean =>
  "context" in app.renderer && app.renderer.context.isLost;

/** Never `destroy(true)`: that releases Pixi's pools, which the console's stage shares with the map (`console-stage.ts`). */
const release = (app: Application): void =>
  app.destroy(
    {
      removeView: true,
    },
    {
      children: true,
    },
  );

/** An application already started, or nothing when none is idle. A lost context is dropped and the next one tried. */
export function takeIdleApp(size: Size): Application | undefined {
  for (let app = idle.pop(); app !== undefined; app = idle.pop()) {
    if (lost(app)) {
      release(app);
      continue;
    }
    // The screen it was last on may have another density.
    const resolution = resolutionNow();
    if (app.renderer.resolution !== resolution) app.renderer.resolution = resolution;
    app.renderer.resize(Math.max(1, size.width), Math.max(1, size.height));
    return app;
  }
  return undefined;
}

/** A new application on a new context. With no WebGL Pixi starts its 2D renderer, which cannot draw these meshes: that one is not kept. */
export async function startApp(size: Size): Promise<Application> {
  const app = new Application();
  await app.init({
    preference: "webgl",
    antialias: true,
    backgroundAlpha: 0,
    autoDensity: true,
    resolution: resolutionNow(),
    width: Math.max(1, size.width),
    height: Math.max(1, size.height),
    // No loop of its own: the stage asks for each frame, and starts the ticker only while something eases.
    autoStart: false,
    sharedTicker: false,
    powerPreference: "low-power",
    eventFeatures: {
      move: false,
      globalMove: false,
      click: false,
      wheel: false,
    },
  });
  // Starting a renderer starts Pixi's system ticker, which then asks for an animation frame every frame for as long as one lives, and an
  // idle map must ask for none. It only sweeps unused textures, which a handful of name labels do not need. A map that was dropped while
  // it started starts it too, so this runs on that path as well.
  Ticker.system.stop();
  return app;
}

/** Hands an application back: its scene is taken apart and its canvas lifted out of the page, then it waits for the next map. */
export function giveBackApp(app: Application): void {
  if (app.renderer.type !== RendererType.WEBGL || lost(app) || idle.length >= IDLE_MAX) {
    release(app);
    return;
  }
  app.ticker.stop();
  app.ticker.maxFPS = 0;
  for (const child of app.stage.removeChildren()) {
    child.destroy({
      children: true,
    });
  }
  // Drawn empty once, so the next map's canvas does not show this one's last picture before its own first frame.
  app.render();
  app.canvas.remove();
  idle.push(app);
}

/** The idle applications, for a test to count and to empty. */
export const idleApps = (): Application[] => idle;
