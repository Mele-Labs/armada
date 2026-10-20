// What a sketch is shared as: Mermaid text, an SVG picture and a web page. The transforms are
// `packages/components/src/compositions/SketchScene/export-*.ts`; the canvas that feeds them is
// `SketchScene.stories.tsx`'s to prove.

import { describe, expect, it } from "vitest";

import { sketchHtml } from "@armada/components/src/compositions/SketchScene/export-html";
import { sceneMermaid } from "@armada/components/src/compositions/SketchScene/export-mermaid";
import { DEFAULT_METRICS, frameOf, sketchSvg, stepPlan, wrapBody } from "@armada/components/src/compositions/SketchScene/export-svg";
import { parseScene } from "@armada/components/src/compositions/SketchScene/scene";
import { sketchFileName } from "@armada/components/src/compositions/SketchScene/share";

import { HUB, TOKENS, geometryOf } from "./sketch-share-fixture";

const svgOf = (standalone: boolean) =>
  sketchSvg({ scene: HUB, geometry: geometryOf(HUB), tokens: TOKENS, icons: { globe: '<circle cx="12" cy="12" r="10"/>', server: '<rect width="20" height="8" x="2" y="2"/>' }, title: "How data moves", standalone });

describe("Copy as Mermaid", () => {
  it("is a flowchart of the boxes, the group they sit in and the labelled arrows, labels escaped", () => {
    expect(sceneMermaid(HUB)).toBe(
      [
        "flowchart LR",
        '  subgraph n1["Fleet host"]',
        '    n3["<b>Fleet</b><br/>Rust daemon: fleet + api + ipc"]',
        "  end",
        '  n2["<b>Bridge</b><br/>Electron, apps/desktop.<br/>One connection, never talks to a Drone"]',
        `  n4["<b>Check</b><br/>The repo's own Check"]`,
        '  n2 -- "WS /jobs/:id/observe and run observe, HTTP for commands" --> n3',
        '  n3 -- "code: #quot;advances#quot; (retry) | fails" --> n4',
        "  n3 --> n4",
        "",
      ].join("\n"),
    );
  });

  it("keeps a ground with nothing in it out, and gives no box a scene id that Mermaid reads as a word", () => {
    const text = sceneMermaid(parseScene({ nodes: [{ id: "end", kind: "box", x: 0, y: 0, title: "End" }, { id: "g", kind: "group", x: 500, y: 500, w: 10, h: 10, title: "Empty" }], edges: [] })!);
    expect(text).not.toContain("Empty");
    expect(text).not.toMatch(/\bend\[/);
    expect(text).toContain('n1["<b>End</b>"]');
  });

  it("turns a character Mermaid would read into an entity, `#` first so the entities survive", () => {
    const text = sceneMermaid(parseScene({ nodes: [{ id: "a", kind: "box", x: 0, y: 0, body: "a#b & <c> `d` \"e\"" }], edges: [] })!);
    expect(text).toContain('n1["a#35;b #amp; #lt;c#gt; #96;d#96; #quot;e#quot;"]');
  });

  it("nests a group in the one that holds it, and joins a ground an arrow ends at", () => {
    const text = sceneMermaid(
      parseScene({
        nodes: [
          { id: "outer", kind: "group", x: 0, y: 0, w: 600, h: 400, title: "Outer" },
          { id: "inner", kind: "lane", x: 20, y: 20, w: 300, h: 200, title: "Inner" },
          { id: "box", kind: "box", x: 40, y: 60, w: 100, h: 50, title: "Box" },
          { id: "alone", kind: "lane", x: 900, y: 900, w: 100, h: 50, title: "Alone" },
        ],
        edges: [{ id: "x", from: "box", to: "alone", label: "to" }],
      })!,
    );
    expect(text).toBe(
      ['flowchart LR', '  subgraph n1["Outer"]', '    subgraph n2["Inner"]', '      n3["<b>Box</b>"]', "    end", "  end", '  n4["<b>Alone</b>"]', '  n3 -- "to" --> n4', ""].join("\n"),
    );
  });
});

describe("Save as image", () => {
  it("holds every box's title and body, and every arrow's words", () => {
    const svg = svgOf(true);
    for (const node of HUB.nodes) {
      if (node.title !== undefined) expect(svg).toContain(`>${node.title}</text>`);
      if (node.body === undefined) continue;
      const from = svg.indexOf(`data-node="${node.id}"`);
      const next = svg.indexOf("data-node=", from + 1);
      const group = svg.slice(from, next === -1 ? undefined : next);
      const said = [...group.matchAll(/<tspan[^>]*>([^<]*)<\/tspan>/g)].map((one) => one[1]!.replace(/&#39;/g, "'").replace(/&quot;/g, '"').replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&")).join(" ");
      expect(said.replace(/\s+/g, " ").trim()).toBe(node.body.replace(/\s+/g, " ").trim());
    }
    for (const edge of HUB.edges) {
      if (edge.label === undefined) continue;
      expect(svg).toContain(`aria-label="${edge.label.replace(/&/g, "&amp;").replace(/"/g, "&quot;")}"`);
    }
  });

  it("names every glyph and draws it from the registry's own strokes", () => {
    expect(svgOf(true)).toContain('role="img" aria-label="Globe"');
  });

  it("refers to nothing outside itself", () => {
    const svg = svgOf(true).replace('xmlns="http://www.w3.org/2000/svg" ', "");
    expect(svg).not.toMatch(/https?:/);
    expect(svg).not.toMatch(/href=|@import|url\((?!#)/);
    expect(svgOf(false)).not.toContain("xmlns");
  });

  it("is framed to the drawing: every box and every arrow's words sit inside the view box", () => {
    const geometry = geometryOf(HUB);
    const frame = frameOf(HUB, geometry);
    const [x, y, w, h] = /viewBox="([-\d.]+) ([-\d.]+) ([\d.]+) ([\d.]+)"/.exec(svgOf(true))!.slice(1).map(Number) as [number, number, number, number];
    expect({ x, y, w, h }).toEqual(frame);
    for (const r of geometry.rects.values()) {
      expect(r.x).toBeGreaterThanOrEqual(x + DEFAULT_METRICS.pad);
      expect(r.y).toBeGreaterThanOrEqual(y + DEFAULT_METRICS.pad);
      expect(r.x + r.w).toBeLessThanOrEqual(x + w - DEFAULT_METRICS.pad);
      expect(r.y + r.h).toBeLessThanOrEqual(y + h - DEFAULT_METRICS.pad);
    }
    for (const p of geometry.places.values()) {
      expect(p.x - p.w / 2).toBeGreaterThanOrEqual(x);
      expect(p.x + p.w / 2).toBeLessThanOrEqual(x + w);
    }
  });

  it("writes the tokens in, so it needs no stylesheet", () => {
    expect(svgOf(true)).toContain("--bg-sunken: #0b0f13;");
  });

  it("escapes what a title or a body says", () => {
    const scene = parseScene({ nodes: [{ id: "a", kind: "box", x: 0, y: 0, title: "<script>", body: "a & b" }], edges: [] })!;
    const svg = sketchSvg({ scene, geometry: geometryOf(scene), tokens: TOKENS, title: 'Say "hi"', standalone: true });
    expect(svg).not.toContain("<script>");
    expect(svg).toContain("&lt;script&gt;");
    expect(svg).toContain("a &amp; b");
    expect(svg).toContain('aria-label="Say &quot;hi&quot;"');
  });
});

describe("a box's words", () => {
  it("break after a hyphen between two letters, as the browser breaks them, and nowhere else inside a word", () => {
    expect(wrapBody("Own worktree, --strict-mcp-config, no push", 23)).toEqual(["Own worktree, --strict-", "mcp-config, no push"]);
    expect(wrapBody("a-b", 2)).toEqual(["a-", "b"]);
    expect(wrapBody("x --- y", 5)).toEqual(["x ---", "y"]);
  });

  it("keep their own breaks, and cut a word longer than a line", () => {
    expect(wrapBody("one\n\ntwo", 10)).toEqual(["one", "", "two"]);
    expect(wrapBody("abcdefghij", 4)).toEqual(["abcd", "efgh", "ij"]);
  });
});

describe("Save as web page", () => {
  const html = () => sketchHtml({ svg: svgOf(false), title: "How data moves", tokens: TOKENS, steps: stepPlan(HUB), pulse: 1600 });

  it("is one document with no address in it", () => {
    const page = html();
    expect(page.match(/<html/g)).toHaveLength(1);
    expect(page).not.toMatch(/https?:/);
    expect(page).not.toMatch(/<link|<img|src=|href=|@import|url\((?!#)/);
  });

  it("holds the drawing and its steps", () => {
    const page = html();
    expect(page).toContain("<svg ");
    expect(page).toContain('data-node="bridge"');
    const json = /<script type="application\/json" id="sketch-steps">([^<]*)<\/script>/.exec(page)![1]!;
    const data = JSON.parse(json) as { pulse: number; steps: { at: number; nodes: string[]; edges: string[] }[] };
    expect(data.pulse).toBe(1600);
    expect(data.steps).toEqual([
      { at: 2, nodes: ["bridge"], edges: [] },
      { at: 3, nodes: ["fleet"], edges: ["e1"] },
      { at: 5, nodes: ["check"], edges: ["e2"] },
    ]);
  });

  it("cannot be broken out of by a title or a label", () => {
    const scene = parseScene({ nodes: [{ id: "a", kind: "box", x: 0, y: 0, title: "</script><b>" }], edges: [] })!;
    const page = sketchHtml({ svg: "", title: "</script><b>", tokens: {}, steps: stepPlan(scene), pulse: 1000 });
    expect(page.match(/<\/script>/g)).toHaveLength(2);
    expect(page).toContain("<title>&lt;/script&gt;&lt;b&gt;</title>");
  });

  it("carries no controls for a sketch with no order to play", () => {
    const scene = parseScene({ nodes: [{ id: "a", kind: "box", x: 0, y: 0, title: "A" }], edges: [] })!;
    expect(sketchHtml({ svg: "", title: "A", tokens: {}, steps: stepPlan(scene), pulse: 1000 })).not.toContain("<button");
  });
});

describe("the file's name", () => {
  it("is the title with what a file system refuses taken out", () => {
    expect(sketchFileName("How data moves to and from the Fleet", "html")).toBe("How data moves to and from the Fleet.html");
    expect(sketchFileName('WS /jobs/:id "observe"?', "png")).toBe("WS jobs id observe.png");
    expect(sketchFileName("  ..hidden.. ", "svg")).toBe("hidden.svg");
  });

  it("is never empty", () => {
    expect(sketchFileName(undefined, "html")).toBe("Sketch.html");
    expect(sketchFileName("///", "html")).toBe("Sketch.html");
  });
});

describe("a shape and a colour, as Mermaid", () => {
  const scene = parseScene({
    nodes: [
      { id: "a", kind: "box", x: 0, y: 0, title: "Start", shape: "pill", color: "green" },
      { id: "b", kind: "box", x: 300, y: 0, title: "Signed in?", shape: "diamond" },
      { id: "c", kind: "box", x: 600, y: 0, title: "Store", shape: "cylinder" },
      { id: "d", kind: "box", x: 900, y: 0, title: "Build", shape: "hexagon" },
      { id: "e", kind: "box", x: 1200, y: 0, title: "Sign in", shape: "circle" },
    ],
    edges: [{ id: "a-b", from: "a", to: "b", color: "red" }],
  })!;

  it("draws each shape in Mermaid's own", () => {
    const text = sceneMermaid(scene);
    expect(text).toContain('n1(["<b>Start</b>"])');
    expect(text).toContain('n2{"<b>Signed in?</b>"}');
    expect(text).toContain('n3[("<b>Store</b>")]');
    expect(text).toContain('n4{{"<b>Build</b>"}}');
    expect(text).toContain('n5(("<b>Sign in</b>"))');
  });

  it("colours a box's edge and an arrow in the hue it was drawn in", () => {
    const text = sceneMermaid(scene, { green: "#4FAF7C", red: "#E97878" });
    expect(text).toContain("style n1 stroke:#4FAF7C");
    expect(text).toContain("linkStyle 0 stroke:#E97878");
  });
});

describe("a shape and a colour, in the picture", () => {
  const scene = parseScene({
    nodes: [
      { id: "a", kind: "box", x: 0, y: 0, title: "Start", shape: "pill", color: "green" },
      { id: "b", kind: "box", x: 300, y: 0, title: "Signed in?", body: "the cookie", shape: "diamond" },
      { id: "c", kind: "box", x: 600, y: 0, title: "Store", shape: "cylinder" },
    ],
    edges: [{ id: "a-b", from: "a", to: "b", color: "red" }],
  })!;
  const svg = sketchSvg({ scene, geometry: geometryOf(scene), tokens: TOKENS, title: "Sign in", standalone: true });

  it("draws each box in its own outline, with no title band", () => {
    expect(svg).toMatch(/data-node="a"[^>]*data-shape="pill"[^>]*>.*?<rect class="sk-shape"[^>]* rx="/);
    expect(svg).toMatch(/data-node="b"[^>]*>.*?<polygon class="sk-shape"/);
    expect(svg).toMatch(/data-node="c"[^>]*>.*?<path class="sk-shape"/);
    expect(svg).not.toMatch(/data-node="b"[^>]*>(?:(?!data-node).)*class="sk-head"/);
    expect(svg).toMatch(/<text class="sk-name"[^>]*text-anchor="middle"[^>]*>Signed in\?<\/text>/);
  });

  it("carries each part's colour, which the drawing's rules turn into its hue", () => {
    expect(svg).toMatch(/data-node="a"[^>]*data-color="green"/);
    expect(svg).toMatch(/data-edge="a-b"[^>]*data-color="red"/);
    expect(svg).toContain('.sk-node[data-color="green"], .sk-edge[data-color="green"] { --hue: var(--sketch-green); }');
  });
});
