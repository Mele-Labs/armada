// A loop that never ends runs for as long as the window is open, so what it animates is what the
// window costs idle. `transform` and `opacity` are drawn by the compositor with no repaint; anything
// else repaints its element every frame, and the front call's `box-shadow` ring alone held the GPU
// helper at 22% and the renderer at 20% on a window with nothing to do. `docs/contracts/design-system.md`, Motion.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "vitest";

const ROOT = new URL("../../../../", import.meta.url).pathname;
const SOURCES = ["apps/desktop/src", "packages"];
const SKIPPED = new Set(["node_modules", "out", "dist", "storybook-static", "target"]);

/** What the compositor moves without painting. */
const COMPOSITED = new Set(["transform", "opacity", "translate", "scale", "rotate"]);

/**
 * Infinite loops that animate something else, each with why. A new one is refused rather than
 * added here: move it to `transform` or `opacity`, or say in this list why it cannot go.
 */
const ALLOWED: Record<string, string> = {
  "armada-scene-flow": "stroke-dashoffset: dashes marching along an SVG edge's own path; a path has no layer to slide. Runs only while a sketch preview is not playing.",
  "sk-flow": "stroke-dashoffset: the exported sketch's copy of armada-scene-flow, in a string of CSS for a standalone page.",
};

function cssFiles(dir: string, found: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    if (SKIPPED.has(name)) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) cssFiles(path, found);
    else if (name.endsWith(".css")) found.push(path);
  }
  return found;
}

/** The standalone page's CSS lives in a template string; it is read the same way. */
function exportedSketchCss(): string {
  return readFileSync(join(ROOT, "packages/components/src/compositions/SketchScene/export-html.ts"), "utf8");
}

/** Each keyframes name with the properties its steps set, and the names an `infinite` animation uses. */
export function readLoops(css: string): { keyframes: Map<string, Set<string>>; infinite: Set<string> } {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const keyframes = new Map<string, Set<string>>();
  for (const found of text.matchAll(/@keyframes\s+([\w-]+)\s*\{/g)) {
    let depth = 1;
    let at = found.index + found[0].length;
    const start = at;
    while (depth > 0 && at < text.length) {
      depth += (text[at] === "{" ? 1 : 0) - (text[at] === "}" ? 1 : 0);
      at += 1;
    }
    const properties = new Set<string>();
    for (const one of text.slice(start, at - 1).matchAll(/([\w-]+)\s*:/g)) properties.add(one[1]!);
    keyframes.set(found[1]!, properties);
  }
  const infinite = new Set<string>();
  for (const found of text.matchAll(/animation(?:-name)?\s*:([^;}]*)/g)) {
    if (!found[1]!.includes("infinite")) continue;
    for (const word of found[1]!.matchAll(/[a-zA-Z][\w-]*/g)) infinite.add(word[0]);
  }
  return { keyframes, infinite };
}

describe("an infinite animation", () => {
  const sheets = SOURCES.flatMap((one) => cssFiles(join(ROOT, one))).map((file) => readFileSync(file, "utf8"));
  const all = readLoops([...sheets, exportedSketchCss()].join("\n"));
  const offenders = [...all.keyframes]
    .filter(([name]) => all.infinite.has(name))
    .map(([name, properties]) => ({ name, extra: [...properties].filter((one) => !COMPOSITED.has(one) && one !== "--amp") }))
    .filter((one) => one.extra.length > 0);

  test("animates transform and opacity, or is named in the allow-list with its reason", () => {
    const refused = offenders.filter((one) => ALLOWED[one.name] === undefined).map((one) => `${one.name}: ${one.extra.join(", ")}`);
    expect(refused).toEqual([]);
  });

  test("is not allowed once it has stopped being an exception", () => {
    const names = new Set(offenders.map((one) => one.name));
    expect(Object.keys(ALLOWED).filter((one) => !names.has(one))).toEqual([]);
  });

  test("is read from a keyframe's own steps", () => {
    const read = readLoops("@keyframes a { 50% { transform: scale(2); box-shadow: none; } } .x { animation: a 1s linear infinite; } @keyframes b { to { opacity: 0 } } .y { animation: b 1s; }");
    expect([...read.infinite].filter((one) => read.keyframes.has(one))).toEqual(["a"]);
    expect([...read.keyframes.get("a")!]).toEqual(["transform", "box-shadow"]);
  });
});
