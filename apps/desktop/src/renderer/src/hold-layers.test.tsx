// The layers that hold a native view. **The annotate host stays in the document for as long as a dev build runs**, so a walk or a browser page held by it never showed.

import { afterEach, expect, test } from "vitest";

import { holdLayerUp } from "./hold-layers";

const mounted: HTMLElement[] = [];
const mount = (make: (element: HTMLElement) => void): void => {
  const element = document.createElement("div");
  make(element);
  document.body.append(element);
  mounted.push(element);
};

afterEach(() => {
  for (const one of mounted.splice(0)) one.remove();
});

test("the annotation layer's idle host is no layer", () => {
  mount((host) => host.setAttribute("data-armada-annotate", ""));
  expect(holdLayerUp()).toBe(false);
});

test("the annotation layer drawn holds the view", () => {
  mount((layer) => {
    layer.className = "armada-annotate";
    layer.setAttribute("data-armada-annotate", "");
  });
  expect(holdLayerUp()).toBe(true);
});

test("a dialog's scrim holds the view, and nothing up does not", () => {
  expect(holdLayerUp()).toBe(false);
  mount((scrim) => {
    scrim.className = "armada-dialog-scrim";
  });
  expect(holdLayerUp()).toBe(true);
});

test("a layer that holds the view's own frame is its ground, and does not hold it", () => {
  const frame = document.createElement("div");
  mount((sheet) => {
    sheet.className = "armada-job-review";
    sheet.append(frame);
  });
  expect(holdLayerUp(frame)).toBe(false);
  // With no frame told, the same layer still holds, as it did.
  expect(holdLayerUp()).toBe(true);
});

test("a layer over the view's frame holds it, whether or not another layer is its ground", () => {
  const frame = document.createElement("div");
  mount((sheet) => {
    sheet.className = "armada-job-review";
    sheet.append(frame);
  });
  mount((menu) => {
    menu.className = "armada-dropdown-menu__panel";
  });
  expect(holdLayerUp(frame)).toBe(true);
  const apart = document.createElement("div");
  document.body.append(apart);
  mounted.push(apart);
  // A frame in no layer is under every layer that is up.
  expect(holdLayerUp(apart)).toBe(true);
});

// A surface that takes the screen is no layer on the list: it is found by what lies on the frame.
const sizedFrame = (within?: HTMLElement): HTMLElement => {
  const frame = document.createElement("div");
  frame.style.cssText = "position:fixed;left:100px;top:100px;width:400px;height:300px";
  if (within === undefined) mount((own) => own.append(frame));
  else within.append(frame);
  return frame;
};
const zoomed = (make?: (over: HTMLElement) => void): HTMLElement => {
  let surface!: HTMLElement;
  mount((over) => {
    over.style.cssText = "position:fixed;left:0;top:0;width:100vw;height:100vh;z-index:20";
    make?.(over);
    surface = over;
  });
  return surface;
};

test("a frame with nothing over it is not held", () => {
  expect(holdLayerUp(sizedFrame())).toBe(false);
});

test("a pane zoomed over the frame holds it, and restored lets it go", () => {
  const frame = sizedFrame();
  const pane = zoomed();
  expect(holdLayerUp(frame)).toBe(true);
  pane.remove();
  expect(holdLayerUp(frame)).toBe(false);
});

test("the frame's own pane zoomed is not over it", () => {
  let frame!: HTMLElement;
  zoomed((pane) => {
    frame = sizedFrame(pane);
  });
  expect(holdLayerUp(frame)).toBe(false);
});

test("something over only part of the frame holds it where it reaches a point asked", () => {
  const frame = sizedFrame();
  mount((over) => {
    over.style.cssText = "position:fixed;left:400px;top:100px;width:200px;height:300px;z-index:20";
  });
  expect(holdLayerUp(frame)).toBe(true);
});

test("what takes no presses is not found on the frame, which is why the annotation layer is on the list", () => {
  const frame = sizedFrame();
  zoomed((over) => {
    over.style.pointerEvents = "none";
  });
  expect(holdLayerUp(frame)).toBe(false);
});

test("a frame with no size has nothing to cover", () => {
  const frame = document.createElement("div");
  mount((own) => own.append(frame));
  expect(holdLayerUp(frame)).toBe(false);
});
