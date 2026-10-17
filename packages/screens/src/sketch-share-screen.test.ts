// What a sketch with a screen in it is shared as. The transforms are
// `packages/components/src/compositions/SketchScene/export-*.ts`; a browser reads the file back in `sketch-share-screen.test.tsx`.

import { describe, expect, it } from "vitest";

import { sketchHtml } from "@armada/components/src/compositions/SketchScene/export-html";
import { sceneMermaid } from "@armada/components/src/compositions/SketchScene/export-mermaid";
import { screenMarkup } from "@armada/components/src/compositions/SketchScene/export-screen";
import { sketchSvg, stepPlan } from "@armada/components/src/compositions/SketchScene/export-svg";
import { parseScene } from "@armada/components/src/compositions/SketchScene/scene";

import { HUB, SCREENS, TOKENS, geometryOf } from "./sketch-share-fixture";

const CSS = ".armada-wireframe { margin: 0; }";
const svgOf = (standalone: boolean, steps: boolean) =>
  sketchSvg({ scene: SCREENS, geometry: geometryOf(SCREENS), tokens: TOKENS, screens: screenMarkup(SCREENS, () => 28, steps, CSS), title: "Sign in flow", standalone });

describe("Save as image, with a screen in it", () => {
  const svg = svgOf(true, false);

  it("carries the screen as drawn: its words, its parts and its rules, not an outline", () => {
    for (const words of ["Welcome back", "Email", "you@example.com", "Sign in", "Tally"]) expect(svg).toContain(words);
    expect(svg).toContain("<foreignObject");
    expect(svg).toContain("armada-wireframe--button");
    expect(svg).toContain(CSS);
  });

  it("names the screen above its sheet, and puts the sheet where the canvas measured it", () => {
    expect(svg).toContain('<text class="sk-caption" x="0" y="10">Sign in</text>');
    expect(svg).toMatch(/<foreignObject x="0" y="28" width="420" height="292"/);
  });

  it("draws the screen as it stands at rest only, with its modal there", () => {
    expect(svg).toContain('data-at="rest"');
    expect(svg).not.toContain('data-off=""');
    expect(svg).not.toContain("data-hidden");
  });

  it("is a namespace of its own for the page markup and for every picture inside it", () => {
    expect(svg).toContain('<div xmlns="http://www.w3.org/1999/xhtml">');
    const drawn = svg.match(/<svg /g) ?? [];
    const named = svg.match(/<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" /g) ?? [];
    expect(drawn.length).toBeGreaterThan(1);
    expect(named).toHaveLength(drawn.length);
  });

  it("refers to nothing outside itself but the namespaces", () => {
    const bare = svg.replace(/xmlns="http:\/\/www\.w3\.org\/(?:1999\/xhtml|2000\/svg)" ?/g, "");
    expect(bare).not.toMatch(/https?:/);
    expect(bare).not.toMatch(/href=|@import|url\((?!#)/);
  });

  it("falls back to the screen's frame and name when it is handed no markup", () => {
    const plain = sketchSvg({ scene: SCREENS, geometry: geometryOf(SCREENS), tokens: TOKENS, title: "Sign in flow", standalone: true });
    expect(plain).not.toContain("<foreignObject");
    expect(plain).toContain('class="sk-wire-title"');
  });

  it("escapes what a screen says like anything else in the file", () => {
    const scene = parseScene({ nodes: [{ id: "a", kind: "screen", x: 0, y: 0, w: 300, h: 200, title: "<b>&", screen: { type: "button", text: '<script>x</script> & "y"' } }], edges: [] })!;
    const out = sketchSvg({ scene, geometry: geometryOf(scene), tokens: TOKENS, screens: screenMarkup(scene, () => 28, false, ""), title: "T", standalone: true });
    expect(out).not.toContain("<script>");
    expect(out).toContain(">&lt;b&gt;&amp;</text>");
    expect(out).toContain("&lt;script&gt;x&lt;/script&gt; &amp; ");
  });
});

describe("Save as web page, with a screen in it", () => {
  const page = () => sketchHtml({ svg: svgOf(false, true), title: "Sign in flow", tokens: TOKENS, steps: stepPlan(SCREENS), pulse: 1600 });

  it("holds the screen at rest and at each step one of its elements carries, only the rest showing", () => {
    const html = page();
    expect(html).toContain('data-at="rest"');
    expect(html).toContain('data-at="1" data-off=""');
    expect(html).toContain('data-at="2" data-off=""');
    expect(html).not.toContain('data-at="3"');
  });

  it("draws the step's own element ringed, and the modal held back until its step", () => {
    const html = page();
    const view = (at: string) => {
      const from = html.indexOf(`data-at="${at}"`);
      return html.slice(from, html.indexOf("</g>", from));
    };
    expect(view("1")).toContain("data-current");
    expect(view("1")).toContain('data-hidden="true"');
    expect(view("2")).not.toContain("data-hidden");
    expect(view("rest")).not.toContain("data-current");
    expect(view("rest")).not.toContain("data-hidden");
  });

  it("is one document with no address in it", () => {
    const html = page();
    expect(html).not.toMatch(/https?:/);
    expect(html).not.toMatch(/<link|<img|src=|href=|@import|url\((?!#)/);
    expect(html).not.toContain("xmlns");
  });

  it("carries the order of its steps, the screen's included, to the player", () => {
    const json = /<script type="application\/json" id="sketch-steps">([^<]*)<\/script>/.exec(page())![1]!;
    expect((JSON.parse(json) as { steps: unknown[] }).steps).toEqual([
      { at: 1, nodes: [], edges: [], inner: ["login"] },
      { at: 2, nodes: [], edges: [], inner: ["login"] },
      { at: 3, nodes: ["home"], edges: ["go"] },
    ]);
  });
});

describe("the order a screen plays in", () => {
  it("lights a node by its own step, and a screen by none: it rings the element that holds the step", () => {
    expect(stepPlan(SCREENS)).toEqual([
      { at: 1, nodes: [], edges: [], inner: ["login"] },
      { at: 2, nodes: [], edges: [], inner: ["login"] },
      { at: 3, nodes: ["home"], edges: ["go"] },
    ]);
  });

  it("lights a screen with a step of its own as it lights a box, and does not also ring inside it", () => {
    const scene = parseScene({ nodes: [{ id: "s", kind: "screen", x: 0, y: 0, step: 1, screen: { type: "button", text: "Go", step: 1 } }], edges: [] })!;
    expect(stepPlan(scene)).toEqual([{ at: 1, nodes: ["s"], edges: [] }]);
  });

  it("is the same order for a scene with no screen in it", () => {
    expect(stepPlan(HUB).every((one) => !("inner" in one))).toBe(true);
  });
});

describe("Copy as Mermaid, with a screen in it", () => {
  it("names each screen as one node with its title, and nothing of what is drawn in it", () => {
    const text = sceneMermaid(SCREENS);
    expect(text).toContain('n1(["<b>Sign in</b>"])');
    expect(text).toContain('n1 -- "Sign in" --> n2');
    for (const words of ["Welcome back", "Email", "Forgot it?"]) expect(text).not.toContain(words);
  });

  it("falls back to the id for a screen with no title", () => {
    const scene = parseScene({ nodes: [{ id: "checkout", kind: "screen", x: 0, y: 0, screen: { type: "divider" } }], edges: [] })!;
    expect(sceneMermaid(scene)).toContain('n1(["checkout"])');
  });
});
