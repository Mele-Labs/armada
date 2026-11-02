// A `mock` node in a browser: the HTML cleaned into the frame's own document, the elements found again by their keys, and the frame
// drawn with the sheets a host read for it. A browser's to prove, since a frame is a document of its own.

import { expect, test, vi } from "vitest";

import { MockView, NO_STYLESHEET, type MockStyles } from "@armada/components/src/compositions/SketchScene/MockView";
import { elementOfKey, mockMarkup, mockNodes, partKey, partName, partOf } from "@armada/components/src/compositions/SketchScene/mock-frame";
import type { SceneNode } from "@armada/components/src/compositions/SketchScene/scene";

import { mount } from "./mounted";

/** The HTML as nodes of a fresh document, which is what the frame is given. */
function drawn(html: string): Document {
  const doc = document.implementation.createHTMLDocument("");
  doc.body.replaceChildren(...mockNodes(doc, html));
  return doc;
}

test("nothing that runs or loads survives into the frame", () => {
  const doc = drawn(
    [
      '<p onclick="steal()" style="position:fixed" data-part="keep">Kept</p>',
      '<script>steal()</script><style>p { color: red }</style><link rel="stylesheet" href="https://example.test/a.css">',
      '<iframe src="https://example.test"></iframe><object data="https://example.test/x"></object><embed src="https://example.test/y">',
      '<a href="https://example.test/go" onmouseover="steal()">Go</a>',
      '<img src="https://example.test/p.png" srcset="https://example.test/p.png 2x" alt="p">',
      '<form action="https://example.test/post"><button formaction="https://example.test/post">Send</button></form>',
      '<svg><use href="#glyph"></use><use href="https://example.test/sprite.svg#a"></use></svg>',
    ].join(""),
  );
  const html = doc.body.innerHTML;
  expect(doc.querySelector("script, style, link, iframe, object, embed")).toBeNull();
  expect(html).not.toMatch(/https?:/);
  expect(html).not.toMatch(/\son\w+=/);
  expect(html).not.toMatch(/\sstyle=/);
  expect(doc.querySelector('[data-part="keep"]')?.textContent).toBe("Kept");
  expect(doc.querySelector("a")?.textContent).toBe("Go");
  // A fragment alone is kept, for an SVG's `use`.
  expect([...doc.querySelectorAll("use")].map((one) => one.getAttribute("href"))).toEqual(["#glyph", null]);
});

test("a data-icon becomes the registry's glyph at the size it names, keeping its other attributes", () => {
  const doc = drawn('<span><i data-icon="bot" data-size="20" class="lamp"></i><i data-icon="not-a-glyph"></i></span>');
  const glyph = doc.querySelector("svg");
  expect(glyph).not.toBeNull();
  expect(glyph?.getAttribute("width")).toBe("20");
  expect(glyph?.getAttribute("class")).toContain("lamp");
  expect(glyph?.getAttribute("aria-hidden")).toBe("true");
  // A name the registry lacks stays as it was written, and draws nothing.
  expect(doc.querySelector('i[data-icon="not-a-glyph"]')).not.toBeNull();
});

test("a saved file's markup is cleaned by the same rules", () => {
  const text = mockMarkup('<p onclick="x()" style="color:red">Hi</p><script>x()</script><i data-icon="bot"></i>');
  expect(text).toContain("Hi");
  expect(text).toContain("<svg");
  expect(text).not.toMatch(/script|onclick|style=/);
});

test("a part is the nearest element named by data-part, else the nearest with words", () => {
  const doc = drawn('<section data-part="card"><div><span id="lamp">Running</span></div><b id="empty"></b></section><p id="loose"><em id="deep">Words</em></p>');
  const body = doc.body;
  expect(partOf(doc.getElementById("lamp"), body)?.getAttribute("data-part")).toBe("card");
  expect(partOf(doc.getElementById("deep"), body)?.id).toBe("deep");
  expect(partOf(doc.getElementById("loose"), body)?.id).toBe("loose");
  // The ground of the frame is no part.
  expect(partOf(body, body)).toBeUndefined();
  expect(partOf(null, body)).toBeUndefined();
});

test("a part is named by its data-part, else by its words cut short", () => {
  const doc = drawn(`<div data-part="lamp">x</div><p id="long">${"word ".repeat(30)}</p>`);
  expect(partName(doc.querySelector("[data-part]")!)).toBe("lamp");
  const said = partName(doc.getElementById("long")!);
  expect(said.endsWith("…")).toBe(true);
  expect(said.length).toBeLessThanOrEqual(41);
});

test("a part is found again by its key: by name, by its place among the same name, or by its path", () => {
  const doc = drawn('<ul><li data-part="row">One</li><li data-part="row">Two</li></ul><p id="path">Plain</p><h2 data-part="title">T</h2>');
  const rows = [...doc.querySelectorAll('[data-part="row"]')];
  expect(partKey(doc.querySelector('[data-part="title"]')!, doc.body)).toBe("title");
  expect(partKey(rows[0]!, doc.body)).toBe("row[1]");
  expect(partKey(rows[1]!, doc.body)).toBe("row[2]");
  expect(elementOfKey(doc, "row[2]")).toBe(rows[1]);
  expect(elementOfKey(doc, "title")).toBe(doc.querySelector('[data-part="title"]'));
  const path = partKey(doc.getElementById("path")!, doc.body);
  expect(path.startsWith("body > ")).toBe(true);
  expect(elementOfKey(doc, path)).toBe(doc.getElementById("path"));
  // A key the frame no longer holds finds nothing, and a path that is not a selector does not throw.
  expect(elementOfKey(doc, "gone")).toBeUndefined();
  expect(elementOfKey(doc, "body >>> nonsense")).toBeUndefined();
});

const NODE: SceneNode = { id: "card", kind: "mock", x: 0, y: 0, w: 320, title: "Job card", html: '<div class="lamp" data-part="lamp">Running</div>', css: ".lamp { color: rgb(255, 0, 0); }" };

const STYLED: MockStyles = {
  css: [".lamp { font-weight: 700; }"],
  fonts: [],
  drawnWith: ["packages/tokens/tokens.css", "packages/components/src/compositions/JobCard/JobCard.css"],
  root: { "data-theme": "dark" },
};

const frameOf = () => document.querySelector<HTMLIFrameElement>("iframe.armada-mock__frame");

test("a frame is drawn with the host's sheets and the node's own rules, and says which files by name", async () => {
  mount(<MockView node={NODE} resolve={async () => STYLED} parts={[]} />);
  await vi.waitFor(() => expect(document.body.textContent).toContain("tokens.css, JobCard.css"));
  await vi.waitFor(() => {
    const doc = frameOf()?.contentDocument;
    const lamp = doc?.querySelector(".lamp");
    expect(lamp).toBeTruthy();
    const style = frameOf()!.contentWindow!.getComputedStyle(lamp!);
    expect(style.color).toBe("rgb(255, 0, 0)");
    expect(style.fontWeight).toBe("700");
  });
  expect(frameOf()?.contentDocument?.documentElement.getAttribute("data-theme")).toBe("dark");
  expect(frameOf()?.getAttribute("sandbox")).toBe("allow-same-origin");
  expect(document.body.textContent).not.toContain(NO_STYLESHEET);
});

test("the line names the files and not their folders", async () => {
  mount(<MockView node={NODE} resolve={async () => STYLED} parts={[]} />);
  await vi.waitFor(() => expect([...document.querySelectorAll("span")].some((one) => one.textContent === "tokens.css, JobCard.css")).toBe(true));
});

test("a node no stylesheet of the app was used for says so", async () => {
  mount(<MockView node={NODE} resolve={async () => ({ css: [], fonts: [], drawnWith: [] })} parts={[]} />);
  await vi.waitFor(() => expect(document.body.textContent).toContain(NO_STYLESHEET));
});

test("a read that failed says the same, and the frame still draws the node's own rules", async () => {
  mount(<MockView node={NODE} resolve={() => Promise.reject(new Error("no"))} parts={[]} />);
  await vi.waitFor(() => expect(document.body.textContent).toContain(NO_STYLESHEET));
  await vi.waitFor(() => {
    const lamp = frameOf()?.contentDocument?.querySelector(".lamp");
    expect(lamp && frameOf()!.contentWindow!.getComputedStyle(lamp).color).toBe("rgb(255, 0, 0)");
  });
});

test("the host is asked with the node's id, the paths it named and the set Fleet kept", async () => {
  const resolve = vi.fn(async () => STYLED);
  mount(<MockView node={{ ...NODE, styles: ["a.css"], kept: "0123456789abcdef0123456789abcdef" }} resolve={resolve} parts={[]} />);
  await vi.waitFor(() => expect(resolve).toHaveBeenCalled());
  expect(resolve).toHaveBeenCalledWith({ id: "card", styles: ["a.css"], kept: "0123456789abcdef0123456789abcdef" });
});
