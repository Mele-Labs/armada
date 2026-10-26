// A Job's document saved as one web page: the sealed page with its CSS inline, each sketch drawn into it as markup by the sketch pane's own
// export, the theme's values written in, and nothing that runs or loads.

import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, test, vi } from "vitest";
import { SketchScene } from "@armada/components";

import "../styles/index.css";
import { readPage } from "./document-page";
import { savedPage, valuesOf, variablesIn } from "./document-save";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const SCENE = {
  nodes: [
    { id: "a", kind: "box", x: 0, y: 0, title: "Caller", body: "append(batch)" },
    { id: "b", kind: "box", x: 320, y: 0, title: "Writer", body: "write(batch)" },
  ],
  edges: [{ id: "a-b", from: "a", to: "b", label: "append" }],
};

const SOURCE = `<!doctype html><html><head><title>Reads &amp; writes</title><link rel="stylesheet" href="armada-kit.css"><style>.note { color: var(--status-awaiting-review); }</style></head>
<body><h1>Reads &amp; writes</h1><p class="note">Both go through one writer.</p>
<h2>Flow</h2>
<armada-sketch title="Flow"><script type="application/json">${JSON.stringify(SCENE)}</script></armada-sketch>
<script>steal()</script><p onclick="steal()">After</p></body></html>`;

/** The sketch as the pane draws it, once its boxes are measured. */
async function drawn(): Promise<string> {
  const host = document.createElement("div");
  host.style.cssText = "inline-size: 900px; block-size: 400px";
  document.body.append(host);
  const exporter: { current: (() => string | undefined) | null } = { current: null };
  const root = createRoot(host);
  act(() => root.render(<SketchScene scene={SCENE} title="Flow" exporter={exporter} />));
  let svg: string | undefined;
  await vi.waitFor(() => {
    svg = exporter.current?.();
    expect(svg).toContain("Writer");
  });
  act(() => root.unmount());
  host.remove();
  return svg!;
}

test("the saved page holds the sketch as markup and the theme's values, and no script, handler or address", async () => {
  const svg = await drawn();
  const page = readPage(SOURCE);
  const probe = document.createElement("div");
  probe.className = "armada-doc__page";
  document.body.append(probe);
  const html = savedPage(page, [svg], valuesOf(page, probe));
  probe.remove();

  const saved = new DOMParser().parseFromString(html, "text/html");
  expect(saved.title).toBe("Reads & writes");
  // The sketch is in the file as an svg with the box words in it, in the place its mount stood.
  const mount = saved.querySelector(".armada-doc__sketch-mount");
  expect(mount?.querySelector("svg")?.textContent).toContain("Caller");
  expect(mount?.querySelector("svg")?.textContent).toContain("Writer");
  expect(saved.querySelector("figcaption")?.textContent).toBe("Flow");
  // The page's words and CSS came along, and the kit's.
  expect(saved.body.textContent).toContain("Both go through one writer.");
  expect(saved.querySelector("style")?.textContent).toContain(".armada-doc__page");
  // The theme's values are written in: every variable the rules read has a value, and the file reads none of Bridge's stylesheets.
  const css = saved.querySelector("style")?.textContent ?? "";
  expect(css).toMatch(/:root \{[^}]*--fg-default: [^;]+;/);
  expect(css).toMatch(/--status-awaiting-review: [^;]+;/);
  for (const name of variablesIn(page.css)) expect(css).toContain(`${name}:`);
  // Nothing runs, and nothing is fetched.
  expect(saved.querySelector("script")).toBeNull();
  expect(saved.querySelector("link")).toBeNull();
  expect(saved.querySelector("[onclick], [onload]")).toBeNull();
  expect(html).not.toMatch(/steal|<script|\son\w+=|href="http|src=|@import|url\((?!#)/i);
  expect(saved.querySelector('meta[http-equiv="Content-Security-Policy"]')?.getAttribute("content")).toBe("default-src 'none'; style-src 'unsafe-inline'");
});

test("a sketch that gave no markup stands as its caption", () => {
  const page = readPage(SOURCE);
  const saved = new DOMParser().parseFromString(savedPage(page, [undefined], {}), "text/html");
  expect(saved.querySelector(".armada-doc__sketch-mount")?.children).toHaveLength(0);
  expect(saved.querySelector("figcaption")?.textContent).toBe("Flow");
});
