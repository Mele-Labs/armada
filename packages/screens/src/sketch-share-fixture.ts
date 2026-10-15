// The scene and geometry the sharing tests draw: a hub with a group, labelled arrows and an order to play in.

import { placeLabels, type Rect } from "@armada/components/src/compositions/SketchScene/edge-labels";
import { DEFAULT_METRICS, nodeHeight, type ExportGeometry } from "@armada/components/src/compositions/SketchScene/export-svg";
import { parseScene, type Scene } from "@armada/components/src/compositions/SketchScene/scene";

/** The hub an agent drew for how data moves through Fleet: converging labelled arrows, and a group round two boxes. */
export const HUB: Scene = parseScene({
  nodes: [
    { id: "g", kind: "group", x: -40, y: 260, w: 340, h: 200, title: "Fleet host" },
    { id: "bridge", kind: "box", x: 0, y: 0, w: 240, h: 100, title: "Bridge", icon: "globe", body: "Electron, apps/desktop.\nOne connection, never talks to a Drone", step: 2 },
    { id: "fleet", kind: "box", x: 0, y: 300, w: 240, h: 100, title: "Fleet", icon: "server", body: "Rust daemon: fleet + api + ipc", step: 3 },
    { id: "check", kind: "box", x: 420, y: 300, w: 240, h: 100, title: "Check", body: "The repo's own Check", step: 5 },
    { id: "empty", kind: "lane", x: -40, y: 600, w: 700, h: 60, title: "Nothing here" },
  ],
  edges: [
    { id: "e1", from: "bridge", to: "fleet", label: "WS /jobs/:id/observe and run observe, HTTP for commands", step: 3 },
    { id: "e2", from: "fleet", to: "check", label: 'code: "advances" (retry) | fails', step: 5 },
    { id: "e3", from: "fleet", to: "check", flow: false },
  ],
})!;

/** A straight bezier between the facing sides of two boxes, which is all the geometry a test needs. */
export function geometryOf(scene: Scene): ExportGeometry {
  const rects = new Map<string, Rect>();
  for (const node of scene.nodes) {
    const w = node.w ?? 240;
    rects.set(node.id, { x: node.x, y: node.y, w, h: nodeHeight(node, w, DEFAULT_METRICS) });
  }
  const paths = new Map<string, string>();
  for (const edge of scene.edges) {
    const a = rects.get(edge.from)!;
    const b = rects.get(edge.to)!;
    const [x0, y0, x1, y1] = [a.x + a.w / 2, a.y + a.h, b.x + b.w / 2, b.y];
    paths.set(edge.id, `M${String(x0)},${String(y0)} C${String(x0)},${String((y0 + y1) / 2)} ${String(x1)},${String((y0 + y1) / 2)} ${String(x1)},${String(y1)}`);
  }
  const boxes = scene.nodes.filter((one) => one.kind !== "group" && one.kind !== "lane").map((one) => rects.get(one.id)!);
  const places = placeLabels(scene.edges.map((one) => ({ id: one.id, label: one.label, path: paths.get(one.id)! })), boxes, { charW: 7, lineH: 16, pad: 4 });
  return { rects, paths, places };
}


/** Token values as a theme holds them. `--ease` is in because an animation naming a token that is not there does not run. */
export const TOKENS = { "--bg-sunken": "#0b0f13", "--fg-default": "#e6edf3", "--status-running": "#3b82f6", "--duration-pulse": "600ms", "--ease": "ease", "--edge-focus": "#58a6ff", "--space-2": "8px", "--edge-active": "2px" };
