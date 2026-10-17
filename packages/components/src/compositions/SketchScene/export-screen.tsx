// What a `screen` node is exported as. **A screen is drawn by the browser**, so a file carries the screen's own HTML and
// the rules that style it, and a viewer's browser lays it out again: nothing is fetched and nothing is measured twice.
// `export-svg.ts` writes the markup into the drawing; this makes it.

import { renderToStaticMarkup } from "react-dom/server";

import type { ExportScreens } from "./export-svg";
import type { Scene } from "./scene";
import { screenSteps } from "./screen";
import { ScreenView } from "./ScreenView";

/**
 * Each screen of the scene as markup: at rest, and with `steps` at every step one of its elements carries, which
 * only a page that plays them needs. `topOf` is where a screen's sheet starts under its title, as the canvas measured it.
 */
export function screenMarkup(scene: Scene, topOf: (id: string) => number, steps: boolean, css: string): ExportScreens {
  const nodes: Record<string, ExportScreens["nodes"][string]> = {};
  for (const node of scene.nodes) {
    const screen = node.screen;
    if (screen === undefined) continue;
    const at = steps ? [...new Set(screenSteps(screen))] : [];
    nodes[node.id] = {
      top: topOf(node.id),
      rest: renderToStaticMarkup(<ScreenView screen={screen} at={undefined} />),
      steps: Object.fromEntries(at.map((one) => [one, renderToStaticMarkup(<ScreenView screen={screen} at={one} />)])),
    };
  }
  return { css, nodes };
}

/** The kit's rules as the page holds them now, read off it so a file never drifts from the canvas. The kit's sheet has no at-rules. */
export function screenStyles(sheets: StyleSheetList = document.styleSheets): string {
  const kit = /\.armada-(?:screen|wireframe)\b/;
  const out: string[] = [];
  for (const sheet of Array.from(sheets)) {
    try {
      for (const rule of Array.from(sheet.cssRules)) {
        if (rule instanceof CSSStyleRule && kit.test(rule.selectorText) && !rule.selectorText.includes("armada-scene")) out.push(rule.cssText);
      }
    } catch {
      // A sheet from another origin cannot be read, and the kit's is never one.
    }
  }
  return out.join("");
}
