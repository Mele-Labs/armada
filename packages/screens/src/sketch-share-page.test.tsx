// The shared web page, opened as a recipient opens it: a document of its own, with no Armada around it.
// The player runs there, so this is a browser's to prove and not a function's.

import { expect, test } from "vitest";

import { sketchHtml } from "@armada/components/src/compositions/SketchScene/export-html";
import { sketchSvg, stepPlan } from "@armada/components/src/compositions/SketchScene/export-svg";

import { motion } from "./mounted";
import { HUB, TOKENS, geometryOf } from "./sketch-share-fixture";

const PULSE = 400;

/** The page in a frame of its own, once it has loaded. */
async function open(): Promise<Document> {
  const page = sketchHtml({
    svg: sketchSvg({ scene: HUB, geometry: geometryOf(HUB), tokens: TOKENS, title: "How data moves", standalone: false }),
    title: "How data moves",
    tokens: TOKENS,
    steps: stepPlan(HUB),
    pulse: PULSE,
  });
  const frame = document.createElement("iframe");
  frame.style.inlineSize = "1000px";
  frame.style.blockSize = "700px";
  frame.srcdoc = page;
  document.body.append(frame);
  await new Promise<void>((done) => frame.addEventListener("load", () => done(), { once: true }));
  return frame.contentDocument!;
}

const waitUntil = async (check: () => boolean) => {
  for (let at = 0; at < 100 && !check(); at += 1) await new Promise((done) => setTimeout(done, 50));
  expect(check()).toBe(true);
};

test("it plays by itself: a step lights its box, stands the rest back, and a dot crosses its arrow", async () => {
  await motion();
  const doc = await open();
  const lit = () => [...doc.querySelectorAll("[data-node][data-current]")].map((one) => one.getAttribute("data-node"));
  await waitUntil(() => lit().length === 1);
  expect(doc.querySelector("svg")!.hasAttribute("data-playing")).toBe(true);
  expect(doc.querySelector("[data-node='check']")!.hasAttribute("data-dim")).toBe(lit()[0] !== "check");
  // The group is the ground the steps happen on, and does not stand back.
  expect(doc.querySelector("[data-node='g']")!.hasAttribute("data-dim")).toBe(false);

  // Step 3 is the arrow `e1`: its dot runs a travel animation along the arrow's own path.
  await waitUntil(() => doc.querySelector("[data-edge='e1'] .sk-edge__dot") !== null);
  const dot = doc.querySelector("[data-edge='e1'] .sk-edge__dot")!;
  expect((dot as SVGElement).style.offsetPath).toContain("path(");
  const travelling = doc.getAnimations().filter((one) => (one as CSSAnimation).animationName === "sk-travel" && one.playState === "running");
  expect(travelling.length).toBe(1);
  expect(doc.querySelector("[data-edge='e1']")!.hasAttribute("data-active")).toBe(true);

  // It loops: the step after the last is the first again.
  await waitUntil(() => lit()[0] === "check");
  await waitUntil(() => lit()[0] === "bridge");
});

test("Stop the steps puts the drawing back as it stands, and Play the steps starts again", async () => {
  await motion();
  const doc = await open();
  const play = doc.getElementById("sketch-play") as HTMLButtonElement;
  await waitUntil(() => play.textContent === "Stop the steps");
  play.click();
  expect(play.textContent).toBe("Play the steps");
  expect(doc.querySelectorAll("[data-current], [data-dim], .sk-edge__dot")).toHaveLength(0);
  expect(doc.querySelector("svg")!.hasAttribute("data-playing")).toBe(false);
  play.click();
  await waitUntil(() => doc.querySelectorAll("[data-current]").length === 1);
});

test("with reduced motion it plays nothing, and the button steps one at a time", async () => {
  const doc = await open();
  const play = doc.getElementById("sketch-play") as HTMLButtonElement;
  expect(play.textContent).toBe("Next step");
  await new Promise((done) => setTimeout(done, PULSE * 2));
  expect(doc.querySelectorAll("[data-current]")).toHaveLength(0);
  play.click();
  expect(doc.querySelector("[data-node='bridge']")!.hasAttribute("data-current")).toBe(true);
  play.click();
  expect(doc.querySelector("[data-node='fleet']")!.hasAttribute("data-current")).toBe(true);
  play.click();
  play.click();
  expect(doc.querySelectorAll("[data-current]")).toHaveLength(0);
});

test("the arrows' words keep their names in the page", async () => {
  const doc = await open();
  expect(doc.querySelector("[role='img'][aria-label^='WS /jobs/:id/observe']")).not.toBeNull();
  expect(doc.querySelector("h1")!.textContent).toBe("How data moves");
});
