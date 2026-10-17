// A sketch with a screen in it, shared and opened as a recipient opens it: a picture a browser reads as XML and draws, and a
// page that plays the screen's steps. A browser's to prove, since the file is drawn by the browser reading it.

import { expect, test } from "vitest";

import { sketchHtml } from "@armada/components/src/compositions/SketchScene/export-html";
import { screenMarkup, screenStyles } from "@armada/components/src/compositions/SketchScene/export-screen";
import { EXPORT_TOKENS, frameOf, sketchSvg, stepPlan } from "@armada/components/src/compositions/SketchScene/export-svg";
import { svgToPng } from "@armada/components/src/compositions/SketchScene/SketchShare";

import { motion } from "./mounted";
import { SCREENS, geometryOf } from "./sketch-share-fixture";

const XHTML = "http://www.w3.org/1999/xhtml";
const SVG = "http://www.w3.org/2000/svg";

/** The theme's token values on this page, as the canvas reads them to make an export. */
const tokens = (): Record<string, string> => {
  const style = getComputedStyle(document.documentElement);
  return Object.fromEntries(EXPORT_TOKENS.map((name) => [name, style.getPropertyValue(name).trim()]));
};

const svgOf = (standalone: boolean, steps: boolean, withScreens = true) =>
  sketchSvg({
    scene: SCREENS,
    geometry: geometryOf(SCREENS),
    tokens: tokens(),
    ...(withScreens ? { screens: screenMarkup(SCREENS, () => 28, steps, screenStyles()) } : {}),
    title: "Sign in flow",
    standalone,
  });

/** How many pixels of a picture are not the ground it is drawn on, which is the colour of its corner. */
async function inked(png: Uint8Array): Promise<number> {
  const bitmap = await createImageBitmap(new Blob([png as BlobPart], { type: "image/png" }));
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const context = canvas.getContext("2d")!;
  context.drawImage(bitmap, 0, 0);
  const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
  let count = 0;
  for (let at = 0; at < data.length; at += 4) {
    if (data[at] !== data[0] || data[at + 1] !== data[1] || data[at + 2] !== data[2]) count += 1;
  }
  return count;
}

test("Save as image is XML a browser reads, with the screen's markup and pictures in their own namespaces", () => {
  const doc = new DOMParser().parseFromString(svgOf(true, false), "image/svg+xml");
  expect(doc.querySelector("parsererror")).toBeNull();
  const button = doc.querySelector(".armada-wireframe--button")!;
  expect(button.namespaceURI).toBe(XHTML);
  expect(button.querySelector("svg")!.namespaceURI).toBe(SVG);
  expect(doc.querySelector("foreignObject")!.parentElement!.namespaceURI).toBe(SVG);
});

test("Save as image draws the sheet, where a picture handed no markup draws its outline", async () => {
  const frame = frameOf(SCREENS, geometryOf(SCREENS));
  const drawn = await inked(await svgToPng(svgOf(true, false), frame.w, frame.h, 1));
  const outline = await inked(await svgToPng(svgOf(true, false, false), frame.w, frame.h, 1));
  // The sheet is 420 by 292, and nearly all of it is ink.
  expect(drawn - outline).toBeGreaterThan(80_000);
});

/** The page in a frame of its own, once it has loaded. */
async function open(): Promise<Document> {
  const page = sketchHtml({ svg: svgOf(false, true), title: "Sign in flow", tokens: tokens(), steps: stepPlan(SCREENS), pulse: 400 });
  const frame = document.createElement("iframe");
  frame.style.inlineSize = "1100px";
  frame.style.blockSize = "700px";
  frame.srcdoc = page;
  document.body.append(frame);
  await new Promise<void>((done) => frame.addEventListener("load", () => done(), { once: true }));
  return frame.contentDocument!;
}

test("the page plays a screen's steps: each rings its element on the screen as it stands then, and the rest stands back", async () => {
  // Reduced motion is on in this project, so the button steps one at a time.
  const doc = await open();
  const play = doc.getElementById("sketch-play") as HTMLButtonElement;
  const shown = () => [...doc.querySelectorAll<SVGElement>("[data-node='login'] .sk-screen")].filter((one) => getComputedStyle(one).display !== "none").map((one) => one.getAttribute("data-at"));
  const flag = (id: string, name: string) => doc.querySelector(`[data-node='${id}']`)!.hasAttribute(name);
  expect(shown()).toEqual(["rest"]);

  play.click();
  expect(shown()).toEqual(["1"]);
  expect(flag("login", "data-dim")).toBe(false);
  expect(flag("login", "data-current")).toBe(false);
  expect(flag("home", "data-dim")).toBe(true);
  expect(doc.querySelector("[data-node='login'] [data-at='1'] .armada-wireframe--button")!.hasAttribute("data-current")).toBe(true);
  expect(getComputedStyle(doc.querySelector("[data-node='login'] [data-at='1'] .armada-wireframe--modal")!).visibility).toBe("hidden");

  play.click();
  expect(shown()).toEqual(["2"]);
  expect(getComputedStyle(doc.querySelector("[data-node='login'] [data-at='2'] .armada-wireframe--modal")!).visibility).toBe("visible");
  expect(doc.querySelector("[data-node='login'] [data-at='2'] .armada-wireframe__dialog")!.hasAttribute("data-current")).toBe(true);

  play.click();
  expect(shown()).toEqual(["rest"]);
  expect(flag("home", "data-current")).toBe(true);
  expect(flag("login", "data-dim")).toBe(true);

  play.click();
  expect(shown()).toEqual(["rest"]);
  expect(doc.querySelectorAll("[data-node][data-current], [data-node][data-dim], [data-edge][data-dim]")).toHaveLength(0);
});

test("the page plays itself through a screen's steps and back to the first", async () => {
  await motion();
  const doc = await open();
  const seen = new Set<string>();
  for (let at = 0; at < 100 && seen.size < 3; at += 1) {
    for (const view of doc.querySelectorAll("[data-node='login'] .sk-screen")) if (getComputedStyle(view).display !== "none") seen.add(view.getAttribute("data-at")!);
    await new Promise((done) => setTimeout(done, 50));
  }
  expect([...seen].sort()).toEqual(["1", "2", "rest"]);
});
