// What a scene says about a screen: the `screen` node and the layout tree it carries. The format is
// `packages/components/src/compositions/SketchScene/screen.ts`; Fleet's validator for `show_sketch` has to agree with it.

import { describe, expect, it } from "vitest";

import { parseScene, type Scene } from "@armada/components/src/compositions/SketchScene/scene";
import { childrenOf, screenSteps, type ScreenElement } from "@armada/components/src/compositions/SketchScene/screen";

/** One of every element type, each with the fields that make it more than its bare type. */
const EVERY: unknown = {
  type: "window",
  chrome: "browser",
  title: "Team",
  url: "app.example/team",
  children: [
    { type: "nav", brand: "Example", items: ["Projects", { text: "Team", icon: "search" }, { icon: "bell" }], active: "Team", actions: [{ icon: "plus" }, "Help"] },
    {
      type: "row",
      gap: "md",
      pad: "sm",
      border: true,
      size: "fill",
      align: "stretch",
      children: [
        { type: "nav", vertical: true, brand: "Settings", items: ["Team"] },
        {
          type: "column",
          size: 2,
          children: [
            { type: "panel", title: "Members", icon: "folder", actions: ["Add"], gap: "sm", pad: "lg", children: [{ type: "text", style: "heading", text: "Members" }] },
            { type: "tabs", items: ["All", "Admins"], active: "All" },
            { type: "list", rows: [{ text: "Ada", meta: "Owner", icon: "check", selected: true }, { text: "Grace" }] },
            { type: "table", columns: ["Name", "Role"], rows: [["Ada", "Owner"]] },
            { type: "grid", cols: 3, children: [{ type: "image", label: "Cover" }, { type: "avatar", text: "A" }, { type: "badge", text: "New" }] },
            { type: "button", text: "Save", variant: "primary", icon: "check" },
            { type: "field", label: "Name", kind: "select", value: "Ada", placeholder: "Who" },
            { type: "text", style: "caption", lines: 3 },
            { type: "divider" },
            { type: "spacer", size: "fill" },
            { type: "chat", messages: [{ from: "me", text: "Hi" }, { from: "them", text: "Hello" }] },
          ],
        },
      ],
    },
    { type: "modal", title: "Invite", step: 2, children: [{ type: "field", kind: "input" }] },
  ],
};

const TYPES = ["row", "column", "grid", "window", "nav", "panel", "tabs", "list", "table", "button", "field", "text", "image", "avatar", "badge", "divider", "spacer", "chat", "modal"];

const typesIn = (el: ScreenElement): string[] => [el.type, ...childrenOf(el).flatMap(typesIn)];

const sceneOf = (screen: unknown, over: Record<string, unknown> = {}): unknown => ({
  nodes: [{ id: "s", kind: "screen", x: 0, y: 0, title: "Team", screen, ...over }],
  edges: [],
});

describe("a screen node", () => {
  it("is read with every element type, and keeps the tree whole", () => {
    const scene = parseScene(sceneOf(EVERY));
    const screen = scene?.nodes[0]?.screen;
    expect(screen).toBeDefined();
    expect(new Set(typesIn(screen!))).toEqual(new Set(TYPES));
    expect(screen).toEqual(EVERY);
  });

  it("keeps a width and a step of its own, and reads the steps of the elements in it", () => {
    const scene = parseScene(sceneOf(EVERY, { w: 700, step: 1 }));
    expect(scene?.nodes[0]).toMatchObject({ kind: "screen", w: 700, step: 1 });
    expect(screenSteps(scene!.nodes[0]!.screen!)).toEqual([2]);
  });

  it("is a node that fails the scene when it has no tree, or one that is not a tree", () => {
    expect(parseScene(sceneOf(undefined))).toBeUndefined();
    expect(parseScene(sceneOf("window"))).toBeUndefined();
    expect(parseScene(sceneOf([EVERY]))).toBeUndefined();
    expect(parseScene(sceneOf({ children: [] }))).toBeUndefined();
  });

  it("leaves a tree on any other kind of node unread, so it is not drawn", () => {
    const scene = parseScene({ nodes: [{ id: "a", kind: "box", x: 0, y: 0, title: "A", screen: EVERY }], edges: [] });
    expect(scene?.nodes[0]).toEqual({ id: "a", kind: "box", x: 0, y: 0, title: "A" });
  });

  it("drops a field no element has, and keeps the rest", () => {
    const scene = parseScene(sceneOf({ type: "button", text: "Save", colour: "red" }));
    expect(scene?.nodes[0]?.screen).toEqual({ type: "button", text: "Save" });
  });
});

describe("a tree that is not one fails the whole scene", () => {
  const deep = (leaf: unknown) => ({ type: "window", chrome: "desktop", children: [{ type: "column", children: [{ type: "panel", children: [leaf] }] }] });

  it("on an element type it does not know, at any depth", () => {
    expect(parseScene(sceneOf({ type: "carousel" }))).toBeUndefined();
    expect(parseScene(sceneOf(deep({ type: "carousel" })))).toBeUndefined();
    expect(parseScene(sceneOf(deep({ children: [] })))).toBeUndefined();
    expect(parseScene(sceneOf(deep("button")))).toBeUndefined();
  });

  it("on an icon that is not one of the screen's glyphs, wherever one is named", () => {
    expect(parseScene(sceneOf({ type: "button", text: "Go", icon: "rocket-ship" }))).toBeUndefined();
    expect(parseScene(sceneOf(deep({ type: "button", icon: "not-a-glyph" })))).toBeUndefined();
    expect(parseScene(sceneOf({ type: "nav", items: [{ text: "Home", icon: "not-a-glyph" }] }))).toBeUndefined();
    expect(parseScene(sceneOf({ type: "panel", icon: "not-a-glyph" }))).toBeUndefined();
    expect(parseScene(sceneOf({ type: "list", rows: [{ text: "A", icon: "not-a-glyph" }] }))).toBeUndefined();
  });

  it("on a size that is not auto, fill or a positive number", () => {
    for (const size of ["wide", "", 0, -1, Number.NaN, Number.POSITIVE_INFINITY, null, true]) {
      expect(parseScene(sceneOf({ type: "spacer", size })), String(size)).toBeUndefined();
      expect(parseScene(sceneOf(deep({ type: "button", size }))), String(size)).toBeUndefined();
    }
    for (const size of ["auto", "fill", 1, 0.5, 3]) expect(parseScene(sceneOf({ type: "spacer", size })), String(size)).toBeDefined();
  });

  it("on a choice outside its list, a missing required field, or a number out of range", () => {
    for (const bad of [
      { type: "window", chrome: "tablet", children: [] },
      { type: "window", children: [] },
      { type: "row" },
      { type: "row", gap: "huge", children: [] },
      { type: "row", align: "middle", children: [] },
      { type: "grid", children: [] },
      { type: "grid", cols: 0, children: [] },
      { type: "grid", cols: 13, children: [] },
      { type: "grid", cols: 2.5, children: [] },
      { type: "button", variant: "danger" },
      { type: "field", kind: "slider" },
      { type: "text", style: "giant" },
      { type: "text", lines: 0 },
      { type: "badge" },
      { type: "tabs" },
      { type: "list" },
      { type: "list", rows: [{ meta: "x" }] },
      { type: "table", columns: ["A"] },
      { type: "table", columns: ["A"], rows: [["x", 1]] },
      { type: "chat", messages: [{ from: "you", text: "x" }] },
      { type: "chat", messages: [{ from: "me" }] },
      { type: "modal" },
    ]) {
      expect(parseScene(sceneOf(bad)), JSON.stringify(bad)).toBeUndefined();
    }
  });

  it("and takes the other nodes of the scene with it", () => {
    const scene = { nodes: [{ id: "a", kind: "box", x: 0, y: 0, title: "A" }, { id: "s", kind: "screen", x: 0, y: 0, screen: { type: "carousel" } }], edges: [] };
    expect(parseScene(scene)).toBeUndefined();
  });
});

describe("a scene with no screen in it", () => {
  const OLD = {
    nodes: [
      { id: "g", kind: "group", x: -20, y: -20, w: 400, h: 300, title: "Host" },
      { id: "l", kind: "lane", x: 0, y: 320, w: 400, h: 60, title: "Lane" },
      { id: "a", kind: "box", x: 0, y: 0, w: 200, h: 80, title: "A", body: "Words", icon: "globe", step: 1 },
      { id: "c", kind: "code", x: 220, y: 0, title: "main.rs", lang: "rust", body: "fn main() {}" },
      { id: "w", kind: "wire", x: 0, y: 120, wire: "button", title: "Save" },
      { id: "t", kind: "label", x: 220, y: 120, title: "Note" },
    ],
    edges: [
      { id: "e1", from: "a", to: "c", label: "calls", flow: false, step: 2 },
      { id: "e2", from: "a", to: "w" },
    ],
    strokes: [{ id: "s1", points: [{ x: 0, y: 0 }, { x: 10, y: 10 }] }],
  };

  it("is read exactly as before", () => {
    expect(parseScene(OLD)).toEqual(OLD);
  });

  it("still fails for what it always failed for", () => {
    const nodes = OLD.nodes;
    expect(parseScene({ nodes: [{ id: "a", kind: "panel", x: 0, y: 0 }], edges: [] })).toBeUndefined();
    expect(parseScene({ nodes: [{ id: "a", kind: "box", x: 0, y: 0, icon: "not-a-glyph" }], edges: [] })).toBeUndefined();
    expect(parseScene({ nodes: [{ id: "a", kind: "group", x: 0, y: 0 }], edges: [] })).toBeUndefined();
    expect(parseScene({ nodes, edges: [{ id: "e", from: "a", to: "missing" }] })).toBeUndefined();
    expect(parseScene({ nodes: [...nodes, nodes[0]], edges: [] })).toBeUndefined();
    expect(parseScene({ nodes, edges: [], strokes: [{ id: "s", points: [{ x: "0", y: 0 }] }] })).toBeUndefined();
  });

  it("has no screen on any node", () => {
    const scene = parseScene(OLD) as Scene;
    expect(scene.nodes.every((one) => !("screen" in one))).toBe(true);
  });
});
