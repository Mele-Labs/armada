import { useEffect, useRef } from "react";

import type { NowSketch } from "../NowPanel/NowPanel";
import { draw } from "./sketch-render";

/**
 * A Drone's diagram, drawn where the canvas would be, from the mermaid source it wrote.
 *
 * **The drawing lives in a shadow root** so mermaid's own stylesheet can be applied as a constructed
 * sheet, which the renderer's CSP allows where an inline `<style>` is refused, and so neither the
 * drawing's rules nor the app's reach the other. A source that does not draw leaves the frame empty.
 */
export function SketchPane({ sketch }: { sketch: NowSketch }) {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let current = true;
    const root = host.current?.shadowRoot ?? host.current?.attachShadow({ mode: "open" });
    if (root === undefined) return;
    root.replaceChildren();
    void draw(sketch).then((drawn) => {
      if (!current || drawn === undefined) return;
      const sheet = new CSSStyleSheet();
      sheet.replaceSync(`${drawn.css}\nsvg { max-width: 100%; max-height: 100%; }`);
      root.adoptedStyleSheets = [sheet];
      const parsed = new DOMParser().parseFromString(drawn.svg, "image/svg+xml").documentElement;
      root.replaceChildren(document.importNode(parsed, true));
    });
    return () => {
      current = false;
    };
  }, [sketch.source]);
  return (
    <figure className="armada-sketch" role="img" aria-label="Sketch">
      <div ref={host} className="armada-sketch__drawing" />
    </figure>
  );
}
