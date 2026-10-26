// A Job's document as one file. The sealed page (`document-page.ts`) with its CSS inline, each sketch replaced by the sketch drawn as static
// markup (the sketch pane's own export, `SketchScene`'s `exporter`) and the theme's token values written in, the way a sketch's own web page
// does. **Nothing in it is fetched and nothing in it runs**: no script, no link, no image address. A policy in the file says so again for a
// browser that opens it. The same file is what a PDF is printed from.

import { tokenDeclarations } from "@armada/components/src/compositions/SketchScene/export-svg";

import { PAGE_ROOT, type Page } from "./document-page";

const POLICY = "default-src 'none'; style-src 'unsafe-inline'";

/** The file's own rules, round the page's: the reading column and how a sketch sits in it. The page's CSS comes after and wins. */
const FILE_CSS = `
* { box-sizing: border-box; }
html { background: var(--bg-base); }
body { margin: 0; padding: var(--space-8) var(--space-6); }
.${PAGE_ROOT} { max-inline-size: 80ch; margin-inline: auto; color: var(--fg-default); font-family: var(--font-sans); font-size: var(--text-base); line-height: var(--leading-base); }
.armada-doc__sketch { margin: var(--space-6) 0; }
.armada-doc__sketch-mount svg { display: block; max-inline-size: 100%; block-size: auto; }
@media print { .armada-doc__sketch, table, pre, blockquote { break-inside: avoid; } }
`;

/** The custom properties a stylesheet reads, by name. */
export const variablesIn = (css: string): string[] => [...new Set([...css.matchAll(/var\(\s*(--[\w-]+)/g)].map((one) => one[1]!))];

/**
 * The page as one self-contained HTML file. `drawn` is each sketch's markup by its place in the page: one that gave none (a scene that does not
 * draw) is left as its caption. `values` is each custom property's value in the theme in force, which `valuesOf` reads.
 */
export function savedPage(page: Page, drawn: readonly (string | undefined)[], values: Readonly<Record<string, string>>): string {
  const body = document.createElement("div");
  body.className = PAGE_ROOT;
  for (const one of Array.from(page.root.childNodes)) body.appendChild(one.cloneNode(true));
  for (const figure of Array.from(body.querySelectorAll<HTMLElement>("figure.armada-doc__sketch"))) {
    const svg = drawn[Number(figure.dataset.sketch)];
    if (svg !== undefined) figure.querySelector(".armada-doc__sketch-mount")?.insertAdjacentHTML("beforeend", svg);
  }
  const title = page.title.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  return [
    `<!doctype html>`,
    `<html lang="en">`,
    `<head>`,
    `<meta charset="utf-8">`,
    `<meta http-equiv="Content-Security-Policy" content="${POLICY}">`,
    `<meta name="viewport" content="width=device-width, initial-scale=1">`,
    `<title>${title}</title>`,
    `<style>:root { ${tokenDeclarations(values)} }${FILE_CSS}${page.css}</style>`,
    `</head>`,
    `<body>`,
    body.outerHTML,
    `</body>`,
    `</html>`,
    ``,
  ].join("\n");
}

/** The value of each custom property the file's rules read, as the element has it now. Those the sketches carry for themselves are not needed. */
export function valuesOf(page: Page, from: Element): Record<string, string> {
  const style = getComputedStyle(from);
  return Object.fromEntries(variablesIn(`${FILE_CSS}${page.css}`).map((name) => [name, style.getPropertyValue(name).trim()]));
}
