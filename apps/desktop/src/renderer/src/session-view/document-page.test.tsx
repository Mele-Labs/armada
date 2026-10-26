// A Job's page is sealed when it is read: a chart drawn as inline SVG survives, and nothing in it can run or load.

import { describe, expect, it } from "vitest";

import { readPage } from "./document-page";

const CHART = `<svg viewBox="0 0 120 60" class="chart" role="img" aria-label="Failures by week" onload="steal()">
  <title>Failures by week</title>
  <defs><clipPath id="clip"><rect x="0" y="0" width="120" height="60"/></clipPath></defs>
  <g clip-path="url(#clip)" fill="none" style="position:fixed">
    <rect class="bar" x="10" y="20" width="20" height="40" fill="var(--accent)"/>
    <circle cx="60" cy="30" r="5" stroke="var(--border-default)" stroke-width="2"/>
    <path d="M0 0 L10 10"/><line x1="0" y1="0" x2="5" y2="5"/><polyline points="0,0 5,5"/><polygon points="0,0 5,5 9,0"/><ellipse cx="1" cy="1" rx="2" ry="3"/>
    <text x="10" y="55" text-anchor="middle"><tspan>Mon</tspan></text>
  </g>
</svg>`;

const svgOf = (html: string) => readPage(`<h1>T</h1>${html}`).root.querySelector("svg");
const names = (el: Element | null) => [...(el?.querySelectorAll("*") ?? [])].map((one) => one.localName);

describe("a chart drawn as inline SVG in a Job's page", () => {
  it("keeps the shapes, text, viewBox and presentation attributes", () => {
    const chart = svgOf(CHART);
    expect(chart).not.toBeNull();
    expect(chart?.namespaceURI).toBe("http://www.w3.org/2000/svg");
    expect(chart?.getAttribute("viewBox")).toBe("0 0 120 60");
    expect(chart?.getAttribute("role")).toBe("img");
    expect(chart?.getAttribute("aria-label")).toBe("Failures by week");
    expect(names(chart)).toEqual(["title", "defs", "clippath", "rect", "g", "rect", "circle", "path", "line", "polyline", "polygon", "ellipse", "text", "tspan"].map((one) => (one === "clippath" ? "clipPath" : one)));
    expect(chart?.querySelector("rect.d-bar")?.getAttribute("fill")).toBe("var(--accent)");
    expect(chart?.querySelector("circle")?.getAttribute("stroke-width")).toBe("2");
    expect(chart?.querySelector("tspan")?.textContent).toBe("Mon");
    expect(chart?.getAttribute("class")).toBe("d-chart");
  });

  it("renames the ids a chart refers to in its own clip-path", () => {
    const chart = svgOf(CHART);
    expect(chart?.querySelector("clipPath")?.id).toBe("d-clip");
    expect(chart?.querySelector("g")?.getAttribute("clip-path")).toBe("url(#d-clip)");
  });

  it("drops every handler and style attribute", () => {
    const chart = svgOf(CHART);
    expect(chart?.hasAttribute("onload")).toBe(false);
    expect(chart?.querySelector("g")?.hasAttribute("style")).toBe(false);
  });

  it("drops a script inside the svg with what it holds", () => {
    const chart = svgOf(`<svg viewBox="0 0 1 1"><script>steal()</script><rect width="1" height="1"/></svg>`);
    expect(names(chart)).toEqual(["rect"]);
    expect(chart?.outerHTML).not.toContain("steal");
  });

  it("drops foreignObject, use, image and animation with what they hold", () => {
    const chart = svgOf(
      `<svg viewBox="0 0 1 1"><foreignObject><div><p>embedded</p></div></foreignObject><use href="http://example.com/x.svg#a"/><image href="http://example.com/x.png"/><animate attributeName="x"/><set attributeName="x"/><rect width="1" height="1"/></svg>`,
    );
    expect(names(chart)).toEqual(["rect"]);
    expect(chart?.textContent).not.toContain("embedded");
  });

  it("drops a link with any address and unwraps one without", () => {
    const chart = svgOf(`<svg viewBox="0 0 1 1"><a href="http://example.com"><text>out</text></a><a xlink:href="javascript:steal()"><text>js</text></a><a><text>plain</text></a></svg>`);
    expect(names(chart)).toEqual(["text"]);
    expect(chart?.textContent).toBe("plain");
  });

  it("drops a value that reaches for an address", () => {
    const chart = svgOf(`<svg viewBox="0 0 1 1"><rect width="1" height="1" fill="url(http://example.com/p.svg#x)" stroke="url(#other)" clip-path="url(javascript:steal())"/></svg>`);
    const rect = chart?.querySelector("rect");
    expect(rect?.hasAttribute("fill")).toBe(false);
    expect(rect?.getAttribute("stroke")).toBe("url(#d-other)");
    expect(rect?.hasAttribute("clip-path")).toBe(false);
  });

  it("leaves no svg where the page's svg is not in the svg namespace", () => {
    expect(readPage(`<h1>T</h1><p>words</p>`).root.querySelector("svg")).toBeNull();
  });
});
