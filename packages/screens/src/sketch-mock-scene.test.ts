// A `mock` node as data: how a scene reads one, how a redraw of it is told, what its parts are called, which kept style set a card
// is given, and what a saved file carries for it. The frame itself is drawn by a browser, in `sketch-mock-frame.test.tsx`.

import { describe, expect, it } from "vitest";

import { sketchSvg } from "@armada/components/src/compositions/SketchScene/export-svg";
import { diffScenes, parseScene, partLabel, type Scene } from "@armada/components/src/compositions/SketchScene/scene";

import { keptFrom, keptStylesOfDetail } from "./sessions-wire";
import { TOKENS, geometryOf } from "./sketch-share-fixture";

const HASH = "0123456789abcdef0123456789abcdef";
const OTHER = "fedcba9876543210fedcba9876543210";
const MOCK = { id: "card", kind: "mock", x: 0, y: 0, w: 320, title: "Job card", html: '<div class="card" data-part="lamp">Running</div>', css: ".card { color: red; }", styles: ["packages/tokens/tokens.css"], kept: HASH };

const sceneOf = (node: Record<string, unknown>): Scene => parseScene({ nodes: [{ ...MOCK, ...node }], edges: [] })!;

describe("a mock in a scene", () => {
  it("is read with its html, its own rules, the paths it names and the set Fleet kept", () => {
    const node = sceneOf({}).nodes[0]!;
    expect(node).toMatchObject({ kind: "mock", w: 320, html: MOCK.html, css: MOCK.css, styles: MOCK.styles, kept: HASH });
  });

  it("is refused without html, without a width, or with a kept hash that is not text", () => {
    expect(parseScene({ nodes: [{ ...MOCK, html: "  " }], edges: [] })).toBeUndefined();
    expect(parseScene({ nodes: [{ ...MOCK, w: undefined }], edges: [] })).toBeUndefined();
    expect(parseScene({ nodes: [{ ...MOCK, kept: 7 }], edges: [] })).toBeUndefined();
  });

  it("is told as changed when its html, its rules, its paths or its kept set change, and as the same when none does", () => {
    const before = sceneOf({});
    const against = (node: Record<string, unknown>) => diffScenes(before, sceneOf(node)).nodes.get("card");
    expect(against({})).toBe("same");
    expect(against({ html: "<p>Done</p>" })).toBe("changed");
    expect(against({ css: ".card { color: blue; }" })).toBe("changed");
    expect(against({ styles: [] })).toBe("changed");
    expect(against({ kept: OTHER })).toBe("changed");
  });

  it("is told as added or removed with the scene", () => {
    const empty: Scene = { nodes: [], edges: [] };
    expect(diffScenes(empty, sceneOf({})).nodes.get("card")).toBe("added");
    expect(diffScenes(sceneOf({}), empty).nodes.get("card")).toBe("removed");
  });
});

describe("what an element of a mock is called", () => {
  const scene = sceneOf({});

  it("is the name its author gave it, without the place among the same name", () => {
    expect(partLabel(scene, "card#lamp")).toBe("lamp");
    expect(partLabel(scene, "card#lamp[2]")).toBe("lamp");
  });

  it("is the mock's own name where the element is found by its path", () => {
    expect(partLabel(scene, "card#body > div:nth-child(1)")).toBe("Job card");
  });

  it("is the id itself where no node of the scene is named", () => {
    expect(partLabel(scene, "other#lamp")).toBe("other#lamp");
  });
});

describe("the kept set a thread card is given", () => {
  const call = sceneOf({ kept: undefined });
  const held = sceneOf({});

  it("is taken from the scene Fleet kept, for the node drawn alike", () => {
    expect(keptFrom(call, held).nodes[0]).toMatchObject({ kept: HASH });
  });

  it("is not given to a node redrawn since", () => {
    expect(keptFrom(sceneOf({ kept: undefined, html: "<p>New</p>" }), held).nodes[0]!.kept).toBeUndefined();
    expect(keptFrom(sceneOf({ kept: undefined, css: "p { margin: 0; }" }), held).nodes[0]!.kept).toBeUndefined();
    expect(keptFrom(sceneOf({ kept: undefined, styles: [] }), held).nodes[0]!.kept).toBeUndefined();
  });

  it("leaves a card alone where the kept scene has none, or where the card has one", () => {
    expect(keptFrom(call, undefined)).toBe(call);
    expect(keptFrom(call, sceneOf({ kept: undefined }))).toBe(call);
    expect(keptFrom(sceneOf({ kept: OTHER }), held).nodes[0]!.kept).toBe(OTHER);
  });
});

describe("a document's kept styles", () => {
  it("are the hashes by `<sketch title>#<node id>`", () => {
    expect(keptStylesOfDetail(JSON.stringify({ "Flow#card": HASH }))).toEqual({ "Flow#card": HASH });
  });

  it("leave out a hash that is not 32 hex digits, and read as none for text that is not that", () => {
    expect(keptStylesOfDetail(JSON.stringify({ "Flow#card": HASH, "Flow#bad": "xyz", "Flow#num": 3 }))).toEqual({ "Flow#card": HASH });
    for (const text of [undefined, "", "not json", "[]", "null", JSON.stringify({ a: "xyz" })]) expect(keptStylesOfDetail(text)).toBeUndefined();
  });
});

describe("Save as web page, with a mock in it", () => {
  const scene = sceneOf({});
  const mocks = { card: { top: 28, height: 140, html: '<div class="card" data-part="lamp">Running</div>', css: ":root { --a: 1; }\n.card { color: red; }", root: { "data-theme": "dark" } } };
  const svg = sketchSvg({ scene, geometry: geometryOf(scene), tokens: TOKENS, mocks, title: "Cards", standalone: false });

  it("carries the part's html and the kept rules in a frame of its own, as tall as the canvas drew it", () => {
    expect(svg).toContain("<iframe");
    expect(svg).toContain("Running");
    expect(svg).toContain(".card { color: red; }");
    expect(svg).toMatch(/<foreignObject x="0" y="28" width="320" height="140"/);
    expect(svg).toContain('<text class="sk-caption" x="0" y="10">Job card</text>');
  });

  it("seals the frame, with no permission at all, and gives the document its own policy that fetches nothing", () => {
    expect(svg).toContain('sandbox=""');
    expect(svg).toContain("default-src &#39;none&#39;");
    expect(svg).toContain("data-theme=&quot;dark&quot;");
  });

  it("holds no script, and no address to fetch", () => {
    expect(svg).not.toMatch(/<script/i);
    expect(svg).not.toMatch(/https?:|href=|@import|url\(/);
  });

  it("cannot be closed early by the rules it carries", () => {
    const hostile = sketchSvg({ scene, geometry: geometryOf(scene), tokens: TOKENS, mocks: { card: { ...mocks.card, css: "</style><script>alert(1)</script>" } }, title: "Cards", standalone: false });
    expect(hostile).not.toMatch(/<script/i);
    expect(hostile).not.toContain("</style><script");
  });

  it("is drawn as its frame and name where it is handed no markup, as a picture is", () => {
    const plain = sketchSvg({ scene, geometry: geometryOf(scene), tokens: TOKENS, title: "Cards", standalone: true });
    expect(plain).not.toContain("<iframe");
    expect(plain).toContain('class="sk-wire-title"');
    expect(plain).toContain("Job card");
  });
});
