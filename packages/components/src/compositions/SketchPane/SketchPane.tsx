import type { NowSketch } from "../NowPanel/NowPanel";

/**
 * A Drone's diagram, drawn where the canvas would be. The drawing is the SVG the host hands over;
 * nothing here reads the mermaid source, so the host owns what it trusts into the page.
 */
export function SketchPane({ sketch }: { sketch: NowSketch }) {
  return <figure className="armada-sketch" role="img" aria-label="Sketch" dangerouslySetInnerHTML={{ __html: sketch.svg }} />;
}
