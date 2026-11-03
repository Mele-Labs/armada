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
