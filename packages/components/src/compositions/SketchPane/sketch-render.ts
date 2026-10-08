// Mermaid source to a drawing safe to insert, and the stylesheet that goes with it.
//
// **Strict, and cleaned again here.** Mermaid runs at `securityLevel: "strict"` with HTML labels off,
// and its output is parsed and stripped before it reaches the page: no script, no foreign object, no
// event attribute, no inline `style` attribute, no link that leaves the drawing. **Its stylesheet is
// returned apart**, because the renderer's CSP refuses an inline `<style>`; the pane applies it as a
// constructed sheet instead.

import type { NowSketch } from "../NowPanel/NowPanel";

export type Drawn = { svg: string; css: string };

const FORBIDDEN = ["script", "foreignObject", "iframe", "object", "embed", "image", "use", "animate", "set"];

/** Strip an SVG down to shapes and text, and pull its `<style>` text out. Pure, so it is tested alone. */
export function cleaned(raw: string): Drawn | undefined {
  const doc = new DOMParser().parseFromString(raw, "image/svg+xml");
  const root = doc.documentElement;
  if (root.nodeName !== "svg" || doc.querySelector("parsererror") !== null) return undefined;
  const css = [...root.querySelectorAll("style")].map((one) => one.textContent ?? "").join("\n");
  for (const one of [root, ...root.querySelectorAll("*")]) {
    if (one.parentNode === null) continue;
    if (one.localName === "style" || FORBIDDEN.includes(one.localName)) {
      one.remove();
      continue;
    }
    for (const attr of [...one.attributes]) {
      const name = attr.name.toLowerCase();
      const outward = (name === "href" || name === "xlink:href") && !attr.value.trim().startsWith("#");
      if (name.startsWith("on") || name === "style" || outward) one.removeAttribute(attr.name);
    }
  }
  return { svg: new XMLSerializer().serializeToString(root), css: safeCss(css) };
}

/** Nothing in the sheet may fetch: no import, and no `url()` but a reference into the drawing. */
export function safeCss(css: string): string {
  return css.replace(/@import[^;]*;/gi, "").replace(/url\(\s*(?!['"]?#)[^)]*\)/gi, "none");
}

let counter = 0;
const made = new Map<string, Promise<Drawn | undefined>>();

/** What the drawing reads its colours from, resolved off the tokens so it sits on either ground. */
function theme(): Record<string, string> {
  const style = getComputedStyle(document.documentElement);
  const token = (name: string) => style.getPropertyValue(name).trim();
  return {
    background: token("--bg-sunken"),
    primaryColor: token("--bg-raised"),
    primaryBorderColor: token("--border-default"),
    primaryTextColor: token("--fg-default"),
    secondaryColor: token("--bg-hover"),
    tertiaryColor: token("--bg-sunken"),
    lineColor: token("--fg-muted"),
    textColor: token("--fg-default"),
    edgeLabelBackground: token("--bg-sunken"),
    noteBkgColor: token("--bg-hover"),
    noteTextColor: token("--fg-default"),
    noteBorderColor: token("--border-default"),
    actorBkg: token("--bg-raised"),
    actorBorder: token("--border-default"),
    actorTextColor: token("--fg-default"),
    signalColor: token("--fg-muted"),
    signalTextColor: token("--fg-default"),
    fontFamily: getComputedStyle(document.body).fontFamily,
  };
}

/** Draw one sketch. Kept by source and colours, so hovering back to one draws nothing again. */
export function draw(sketch: NowSketch): Promise<Drawn | undefined> {
  const variables = theme();
  const key = `${JSON.stringify(variables)}\n${sketch.source}`;
  const kept = made.get(key);
  if (kept !== undefined) return kept;
  const pending = (async () => {
    const { default: mermaid } = await import("mermaid");
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      theme: "base",
      htmlLabels: false,
      themeVariables: variables,
      flowchart: { htmlLabels: false, useMaxWidth: false },
      sequence: { useMaxWidth: false },
      state: { useMaxWidth: false },
    });
    if (!(await mermaid.parse(sketch.source, { suppressErrors: true }))) return undefined;
    counter += 1;
    const { svg } = await mermaid.render(`armada-sketch-${counter}`, sketch.source);
    return cleaned(svg);
  })().catch(() => undefined);
  made.set(key, pending);
  return pending;
}
